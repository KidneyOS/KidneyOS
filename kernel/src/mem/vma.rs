use crate::fs::fs_manager::FileSystemID;
use crate::system::unwrap_system;
use crate::vfs::INodeNum;
use crate::KERNEL_ALLOCATOR;
use alloc::collections::BTreeMap;
use kidneyos_shared::mem::{OFFSET, PAGE_FRAME_SIZE};

/// A list of virtual memory areas for a process
#[derive(Debug, Default, Clone)]
pub struct VMAList(BTreeMap<usize, VMA>);

/// A virtual memory area
#[derive(Debug, Clone)]
pub struct VMA {
    info: VMAInfo,
    size: usize,
    writeable: bool,
    // no point in having other permissions since x86 only supports RWX and RX by default.
}

/// Type of VMA and any specific data associated with it
#[derive(Debug)]
pub enum VMAInfo {
    /// This VMA contains the stack
    Stack,
    /// This VMA contains the heap
    Heap,
    /// This VMA contains a memory-mapped file
    ///
    /// `offset` is in units of pages
    MMap {
        fs: FileSystemID,
        inode: INodeNum,
        offset: u32,
    },
}

impl Clone for VMAInfo {
    /// clone VMAInfo on fork
    fn clone(&self) -> Self {
        match self {
            Self::Stack => Self::Stack,
            Self::Heap => Self::Heap,
            Self::MMap { fs, inode, offset } => {
                let fs = *fs;
                let inode = *inode;
                let offset = *offset;
                // increment reference count to inode to allow mmapped closed file to still be read.
                let mut root = unwrap_system().root_filesystem.lock();
                root.increment_inode_ref_count(fs, inode);
                Self::MMap { fs, inode, offset }
            }
        }
    }
}

impl VMA {
    pub fn new(info: VMAInfo, size: usize, writeable: bool) -> Self {
        Self {
            info,
            size,
            writeable,
        }
    }
    pub fn info(&self) -> &VMAInfo {
        &self.info
    }
    pub fn size(&self) -> usize {
        self.size
    }
    pub fn writeable(&self) -> bool {
        self.writeable
    }
    #[must_use]
    unsafe fn install_in_page_table(&self, virt_addr: usize, offset: usize) -> bool {
        debug_assert_eq!(virt_addr % PAGE_FRAME_SIZE, 0);
        debug_assert_eq!(offset % PAGE_FRAME_SIZE, 0);
        let Ok(frame_ptr) = (unsafe { KERNEL_ALLOCATOR.frame_alloc(1) }) else {
            return false;
        };
        let frame_ptr = frame_ptr.as_ptr() as *mut u8;
        let phys_addr = frame_ptr as usize - OFFSET;
        let mut tcb_guard = unwrap_system().threads.running_thread.lock();
        let tcb = tcb_guard.as_mut().expect("no running thread");
        tcb.page_manager
            .map(phys_addr, virt_addr, self.writeable(), true);
        drop(tcb_guard);
        // important we don't use the virtual address here since it may be read-only!
        let data = core::slice::from_raw_parts_mut(frame_ptr, PAGE_FRAME_SIZE);
        match &self.info {
            VMAInfo::Stack | VMAInfo::Heap => {
                // zero memory, to prevent data from being leaked between processes.
                data.fill(0);
                true
            }
            VMAInfo::MMap { fs, inode, offset } => {
                let fs = *fs;
                let inode = *inode;
                let offset = u64::from(*offset) * PAGE_FRAME_SIZE as u64;
                let mut root = unwrap_system().root_filesystem.lock();
                let mut bytes_read = 0;
                while bytes_read < PAGE_FRAME_SIZE {
                    let n = match root.read_direct(
                        fs,
                        inode,
                        offset + bytes_read as u64,
                        &mut data[bytes_read..],
                    ) {
                        Ok(0) => {
                            break;
                        }
                        Ok(n) => n,
                        // Generate page fault if reading data from mmapped file fails.
                        // This seems to be consistent with other OSes (some do a bus error instead)
                        Err(_) => {
                            return false;
                        }
                    };
                    bytes_read += n;
                }
                // if we reached the end of the file, fill the rest of the page with zeros.
                data[bytes_read..].fill(0);
                true
            }
        }
    }
}

impl VMAList {
    /// New empty list of VMAs.
    pub fn new() -> Self {
        Self::default()
    }
    fn vma_at(&self, addr: usize) -> Option<(usize, &VMA)> {
        // find VMA whose address is closest to addr without going over
        let (vma_addr, vma) = self.0.range(..=addr).next_back()?;
        let vma_addr = *vma_addr;
        // check if addr actually lies in the VMA
        if addr >= vma_addr && addr < vma_addr + vma.size {
            Some((vma_addr, vma))
        } else {
            None
        }
    }
    fn is_address_range_free(&self, range: core::ops::Range<usize>) -> bool {
        // make sure there is no VMA whose address is before the start of range, but still
        // overlaps range because of its length
        if self.vma_at(range.start).is_some() {
            return false;
        }
        self.0.range(range.start..range.end).next().is_none()
    }
    /// Install PTE for virtual address `addr`, if possible.
    ///
    /// Returns `false` on failure, e.g. couldn't allocate physical memory, there is no VMA covering `addr`,
    /// couldn't read mmapped file.
    ///
    /// # Safety
    ///
    /// `addr` must be currently unmapped.
    #[must_use]
    pub unsafe fn install_pte(&self, addr: usize) -> bool {
        // round down to page
        let addr = addr & !(PAGE_FRAME_SIZE - 1);
        let Some((vma_addr, vma)) = self.vma_at(addr) else {
            return false;
        };
        vma.install_in_page_table(addr, addr - vma_addr)
    }
    /// Add a VMA to the list.
    ///
    /// `addr` must be a multiple of `PAGE_FRAME_SIZE`. If there is already a VMA anywhere in the address range, returns `false`.
    #[must_use]
    pub fn add_vma(&mut self, vma: VMA, addr: usize) -> bool {
        assert_eq!(addr % PAGE_FRAME_SIZE, 0);
        if !self.is_address_range_free(addr..addr + vma.size) {
            return false;
        }
        self.0.insert(addr, vma);
        true
    }
    pub fn iter(&self) -> impl '_ + Iterator<Item = (usize, &VMA)> {
        self.0.iter().map(|(&k, v)| (k, v))
    }
    // TODO: free physical memory allocated by VMAs on process exit
}
