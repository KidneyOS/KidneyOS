use crate::fs::FileDescriptor;
use crate::KERNEL_ALLOCATOR;
use crate::system::unwrap_system;
use kidneyos_shared::mem::{OFFSET, PAGE_FRAME_SIZE};
use alloc::collections::BTreeMap;

#[derive(Debug, Default, Clone)]
pub struct VMAList(BTreeMap<usize, VMA>);

#[derive(Debug, Clone)]
pub struct VMA {
    info: VMAInfo,
    size: usize,
    writeable: bool,
}

#[derive(Debug)]
pub enum VMAInfo {
    Stack,
    Data,
    MMap { fd: FileDescriptor, offset: u64 },
}

impl Clone for VMAInfo {
    /// clone VMAInfo on fork
    fn clone(&self) -> Self {
        match self {
            Self::Stack => Self::Stack,
            Self::Data => Self::Data,
            Self::MMap { .. } => todo!("increment ref count to mmapped file"),
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
        let phys_addr = frame_ptr.as_ptr() as *const u8 as usize - OFFSET;
        let mut tcb_guard = unwrap_system().threads.running_thread.lock();
        let tcb = tcb_guard.as_mut().expect("no running thread");
        tcb.page_manager.map(phys_addr, virt_addr, self.writeable(), true);
        drop(tcb_guard);
        match self.info {
            VMAInfo::Stack | VMAInfo::Data => {
                // zero memory, to prevent data from being leaked between processes.
                (virt_addr as *mut u8).write_bytes(0, PAGE_FRAME_SIZE);
                true
            }
            VMAInfo::MMap { .. } => todo!()
        }
    }
}

impl VMAList {
    /// New empty list of VMAs.
    pub fn new() -> Self {
        Self::default()
    }
    fn vma_at(&self, addr: usize) -> Option<(usize, &VMA)> {
        let (vma_addr, vma) = self.0.range(..=addr).next()?;
        let vma_addr = *vma_addr;
        if addr >= vma_addr && addr < vma_addr + vma.size {
            Some((vma_addr, vma))
        } else {
            None
        }
    }
    fn is_address_range_free(&self, range: core::ops::Range<usize>) -> bool {
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
    #[must_use]
    pub fn add_vma(&mut self, vma: VMA, addr: usize) -> bool {
        assert_eq!(addr % PAGE_FRAME_SIZE, 0);
        if !self.is_address_range_free(addr..addr + vma.size) {
            return false;
        }
        self.0.insert(addr, vma);
        true
    }
    // TODO: free physical memory allocated by VMAs on process exit
}
