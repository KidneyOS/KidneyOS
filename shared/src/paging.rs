// https://wiki.osdev.org/Paging
// https://wiki.osdev.org/Setting_Up_Paging

use crate::{
    mem::{
        phys::{kernel_data_start, kernel_end, kernel_start, main_stack_top, trampoline_heap_top},
        virt, HUGE_PAGE_SIZE, OFFSET, PAGE_FRAME_SIZE,
    },
    video_memory::{VIDEO_MEMORY_BASE, VIDEO_MEMORY_SIZE},
};
use arbitrary_int::{u10, u12, u20};
use bitbybit::bitfield;
use core::{
    alloc::{Allocator, Layout},
    arch::asm,
    mem::size_of,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};
use lazy_static::lazy_static;

const PAGE_DIRECTORY_LEN: usize = PAGE_FRAME_SIZE / size_of::<PageDirectoryEntry>();

#[derive(Clone)]
#[repr(align(4096))]
struct PageDirectory([PageDirectoryEntry; PAGE_DIRECTORY_LEN]);

impl Default for PageDirectory {
    fn default() -> Self {
        Self([PageDirectoryEntry::default(); PAGE_DIRECTORY_LEN])
    }
}

impl Deref for PageDirectory {
    type Target = [PageDirectoryEntry; PAGE_DIRECTORY_LEN];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PageDirectory {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl PageDirectory {
    fn page_table(
        &self,
        page_directory_index: usize,
        phys_to_alloc_addr_offset: usize,
    ) -> *mut PageTable {
        let page_table_frame = self[page_directory_index].page_table_frame().value() as usize;

        ((page_table_frame * size_of::<PageTable>()) + phys_to_alloc_addr_offset) as *mut PageTable
    }
}

#[bitfield(u32, default = 0)]
struct PageDirectoryEntry {
    #[bit(0, rw)]
    present: bool,
    #[bit(1, rw)]
    read_write: bool,
    #[bit(2, rw)]
    user_supervisor: bool,
    #[bit(3, rw)]
    write_through: bool,
    #[bit(4, rw)]
    cache_disable: bool,
    #[bit(5, rw)]
    accessed: bool,
    #[bit(7, rw)]
    page_size: bool,
    #[bits(12..=31, rw)]
    page_table_frame: u20,
}

const PAGE_TABLE_LEN: usize = PAGE_FRAME_SIZE / size_of::<PageTableEntry>();

#[repr(align(4096))]
struct PageTable([PageTableEntry; PAGE_TABLE_LEN]);

impl Default for PageTable {
    fn default() -> Self {
        Self([PageTableEntry::default(); PAGE_TABLE_LEN])
    }
}

impl Deref for PageTable {
    type Target = [PageTableEntry; PAGE_TABLE_LEN];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PageTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[bitfield(u32, default = 0)]
struct PageTableEntry {
    #[bit(0, rw)]
    present: bool,
    #[bit(1, rw)]
    read_write: bool,
    #[bit(2, rw)]
    user_supervisor: bool,
    #[bit(3, rw)]
    write_through: bool,
    #[bit(4, rw)]
    cache_disable: bool,
    #[bit(5, rw)]
    accessed: bool,
    #[bit(6, rw)]
    dirty: bool,
    #[bit(7, rw)]
    page_attribute_table: bool,
    #[bit(8, rw)]
    global: bool,
    #[bits(12..=31, rw)]
    page_frame: u20,
}

fn virt_parts(virt_addr: usize) -> (usize, usize) {
    #[bitfield(u32)]
    struct VirtualAddress {
        #[bits(22..=31, r)]
        page_directory_index: u10,
        #[bits(12..=21, r)]
        page_table_index: u10,
        #[bits(0..=11, r)]
        offset: u12,
    }

    let virt_addr = VirtualAddress::new_with_raw_value(virt_addr as u32);
    (
        virt_addr.page_directory_index().value() as usize,
        virt_addr.page_table_index().value() as usize,
    )
}

/// Wraps lower-level paging data structures.
pub struct PageManager<A: Allocator> {
    root: NonNull<PageDirectory>,
    alloc: A,
    phys_to_alloc_addr_offset: usize,
}

const PAGE_DIRECTORY_LAYOUT: Layout = Layout::new::<PageDirectory>();
const PAGE_TABLE_LAYOUT: Layout = Layout::new::<PageTable>();

impl<A: Allocator> PageManager<A> {
    pub fn new_in(alloc: A, alloc_addr_to_phys_offset: usize) -> Self {
        let Ok(root_addr) = alloc.allocate(PAGE_DIRECTORY_LAYOUT) else {
            panic!("allocation failed");
        };

        let mut root = root_addr.cast::<PageDirectory>();
        unsafe { *root.as_mut() = PageDirectory::default() };

        Self {
            root,
            alloc,
            phys_to_alloc_addr_offset: alloc_addr_to_phys_offset,
        }
    }

    pub fn from_mapping_ranges_in<R: IntoIterator<Item = MappingRange>>(
        mapping_ranges: R,
        alloc: A,
        alloc_addr_to_phys_offset: usize,
    ) -> Self {
        let mut res = Self::new_in(alloc, alloc_addr_to_phys_offset);

        for MappingRange {
            phys_start,
            virt_start,
            len,
            write,
            user,
        } in mapping_ranges
        {
            // SAFETY: res has not yet been loaded.
            unsafe { res.map_range(phys_start, virt_start, len, write, user) };
        }

        res
    }

    /// Load these page tables into the cr3 CPU register. If paging has not yet
    /// been enabled, `enable` must be called before the page tables will have
    /// any effect.
    ///
    /// # Safety
    ///
    /// Swapping from the previously loaded page tables to these must not cause
    /// any existing pointers to refer to anything they shouldn't.
    pub unsafe fn load(&self) {
        let root_phys_addr = self.root.as_ptr() as usize - self.phys_to_alloc_addr_offset;
        unsafe { asm!("mov cr3, {}", in(reg) root_phys_addr, options(nostack)) };
    }

    /// Returns whether these page tables are loaded.
    pub fn is_loaded(&self) -> bool {
        let current_root: usize;
        unsafe { asm!("mov {}, cr3", out(reg) current_root, options(nomem, nostack)) };
        current_root != self.root.as_ptr() as usize - self.phys_to_alloc_addr_offset
    }

    /// Maps virtual addresses from `virt_addr..(virt_addr + PAGE_FRAME_SIZE)`
    /// to the physical addresses `phys_addr..(phys_addr + PAGE_FRAME_SIZE)`.
    /// `phys_addr` and `virt_addr` must both be page-frame-aligned. In other
    /// words, they must be multiples of `PAGE_FRAME_SIZE`.
    ///
    /// The virtual addresses must not already be mapped. If these page tables
    /// are already loaded, the new mappings are not guaranteed to be recognized
    /// by the CPU until `load` is called again.
    ///
    /// # Safety
    ///
    /// Adding this mapping must not cause any existing pointers to refer to
    /// anything they shouldn't.
    ///
    /// This is still unsafe despite the requirement to call `load` because if
    /// there's nothing in the TLB for the virtual addresses included in this
    /// mapping, the mapping may take effect without the call to `load`.
    pub unsafe fn map(&mut self, phys_addr: usize, virt_addr: usize, write: bool, user: bool) {
        assert_eq!(
            phys_addr % PAGE_FRAME_SIZE,
            0,
            "phys_addr was not page-frame-aligned"
        );
        assert_eq!(
            virt_addr % PAGE_FRAME_SIZE,
            0,
            "virt_addr was not page-frame-aligned"
        );

        let page_directory = self.root.as_mut();
        let (pdi, pti) = virt_parts(virt_addr);

        let page_table = if !page_directory[pdi].present() {
            let Ok(page_table_addr) = self.alloc.allocate(PAGE_TABLE_LAYOUT) else {
                panic!("allocation failed");
            };

            let page_table = page_table_addr.cast::<PageTable>().as_mut();
            *page_table = PageTable::default();

            let page_table_phys_addr =
                page_table_addr.cast::<u8>().as_ptr() as usize - self.phys_to_alloc_addr_offset;
            let page_table_frame = page_table_phys_addr / size_of::<PageTable>();
            page_directory[pdi] = PageDirectoryEntry::default()
                .with_present(true)
                .with_read_write(write)
                .with_user_supervisor(user)
                .with_page_table_frame(u20::new(page_table_frame as u32));
            page_table
        } else {
            // NOTE: For a page to be considered writable, the read_write bit
            // must be set in both the page directory entry, and the page table
            // entry, so it's safe for us to enable things here. Same goes for
            // user_supervisor.
            if write && !page_directory[pdi].read_write() {
                page_directory[pdi] = page_directory[pdi].with_read_write(true);
            }
            if user && !page_directory[pdi].user_supervisor() {
                page_directory[pdi] = page_directory[pdi].with_user_supervisor(true);
            }

            &mut *page_directory.page_table(pdi, self.phys_to_alloc_addr_offset)
        };

        assert!(
            !page_table[pti].present(),
            "virtual address {:#X} was already mapped",
            virt_addr
        );

        let phys_frame = (phys_addr / PAGE_FRAME_SIZE) as u32;
        page_table[pti] = PageTableEntry::default()
            .with_present(true)
            .with_read_write(write)
            .with_user_supervisor(user)
            .with_page_frame(u20::new(phys_frame));
    }

    /// Like map, except with length `HUGE_PAGE_SIZE`. `virt_addr` must have an
    /// alignment of `HUGE_PAGE_SIZE`, but `phys_addr` only needs to be aligned
    /// to `PAGE_FRAME_SIZE`. PSE must be enabled.
    ///
    /// # Safety
    ///
    /// Same as `map`.
    pub unsafe fn huge_map(&mut self, phys_addr: usize, virt_addr: usize, write: bool, user: bool) {
        assert!(*PSE_ENABLED, "PSE was not enabled");
        assert_eq!(
            phys_addr % PAGE_FRAME_SIZE,
            0,
            "phys_addr was not page-frame-aligned"
        );
        assert_eq!(
            virt_addr % HUGE_PAGE_SIZE,
            0,
            "virt_addr was not properly aligned"
        );

        let page_directory = self.root.as_mut();
        let (pdi, _) = virt_parts(virt_addr);

        assert!(
            !page_directory[pdi].present(),
            "virtual address {:#X} was already mapped",
            virt_addr
        );

        page_directory[pdi] = PageDirectoryEntry::default()
            .with_present(true)
            .with_read_write(write)
            .with_user_supervisor(user)
            .with_page_size(true)
            .with_page_table_frame(u20::new((phys_addr / PAGE_FRAME_SIZE) as u32));
    }

    /// Maps virtual addresses from `virt_start..(virt_start + len)` to the
    /// physical addresses `phys_start..(phys_start + len)`. `phys_start` and
    /// `virt_start` must both be page-frame-aligned. In other words, they must
    /// be multiples of `PAGE_FRAME_SIZE`. `len` must also be a multiple of
    /// `PAGE_FRAME_SIZE`.
    ///
    /// The same rules apply with regards to `load` as with `map`.
    ///
    /// # Safety
    ///
    /// Same as `map`.
    pub unsafe fn map_range(
        &mut self,
        phys_start: usize,
        virt_start: usize,
        len: usize,
        write: bool,
        user: bool,
    ) {
        assert_eq!(
            phys_start % PAGE_FRAME_SIZE,
            0,
            "phys_start was not page-frame-aligned"
        );
        assert_eq!(
            virt_start % PAGE_FRAME_SIZE,
            0,
            "virt_start was not page-frame-aligned"
        );
        assert_eq!(
            len % PAGE_FRAME_SIZE,
            0,
            "len was not a multiple of PAGE_FRAME_SIZE"
        );

        let mut phys_addr = phys_start;
        let mut virt_addr = virt_start;
        loop {
            if phys_addr - phys_start >= len {
                break;
            }

            if *PSE_ENABLED
                && virt_addr % HUGE_PAGE_SIZE == 0
                && phys_addr.saturating_add(HUGE_PAGE_SIZE) - phys_start <= len
            {
                self.huge_map(phys_addr, virt_addr, write, user);

                phys_addr = phys_addr.saturating_add(HUGE_PAGE_SIZE);
                virt_addr = virt_addr.saturating_add(HUGE_PAGE_SIZE);

                continue;
            }

            self.map(phys_addr, virt_addr, write, user);

            phys_addr = phys_addr.saturating_add(PAGE_FRAME_SIZE);
            virt_addr = virt_addr.saturating_add(PAGE_FRAME_SIZE);
        }
    }

    /// Like `map_range` except phys_start and virt_start are both `start`.
    ///
    /// # Safety
    ///
    /// Same as `map`.
    pub unsafe fn id_map_range(
        &mut self,
        start: usize,
        frames_len: usize,
        write: bool,
        user: bool,
    ) {
        self.map_range(start, start, frames_len, write, user);
    }

    /// Returns true if this `pointer` is mapped in this PageManager.
    ///
    /// This doesn't return a &PageEntry because it would work strangely with huge pages.
    pub fn is_mapped(&self, pointer: usize) -> bool {
        let (pdi, pti) = virt_parts(pointer);

        let page_directory = unsafe { self.root.as_ref() };

        let entry = &page_directory.0[pdi];

        // Entry is not present, don't bother looking any further.
        if !entry.present() {
            return false;
        }

        // Huge, page, this point is mapped.
        if entry.page_size() {
            return true;
        }

        // A bit unsettling...
        let page_table =
            unsafe { &*page_directory.page_table(pdi, self.phys_to_alloc_addr_offset) };

        page_table.0[pti].present()
    }

    /// Returns true if `count` bytes after `pointer` are mapped in this PageManager.
    pub fn is_range_mapped(&self, pointer: usize, count: usize) -> bool {
        // Align downwards.
        let start = (pointer / PAGE_FRAME_SIZE) * PAGE_FRAME_SIZE;

        // Check if each 4K page is mapped.
        (start..pointer + count)
            .step_by(PAGE_FRAME_SIZE)
            .all(|ptr| self.is_mapped(ptr))
    }
}

impl<A: Allocator + Copy> Clone for PageManager<A> {
    fn clone(&self) -> Self {
        let Ok(root_addr) = self.alloc.allocate(PAGE_DIRECTORY_LAYOUT) else {
            panic!("allocation failed");
        };

        let mut root = root_addr.cast::<PageDirectory>();
        unsafe { *root.as_mut() = (*self.root.as_ptr()).clone() };

        Self { root, ..*self }
    }
}

impl<A: Allocator> Drop for PageManager<A> {
    fn drop(&mut self) {
        assert!(!self.is_loaded(), "page manager dropped while still loaded");

        for pde in unsafe { &self.root.as_ref().0 } {
            if !pde.present() {
                continue;
            }

            let page_table_addr = pde.page_table_frame().value() as usize * size_of::<PageTable>()
                + self.phys_to_alloc_addr_offset;
            let Some(page_table_addr) = NonNull::new(page_table_addr as *mut u8) else {
                panic!("present page directory entry contained null page table address");
            };

            // Huge Pages are not allocated with an allocator.
            // Seems like we might not need to free them.
            if !pde.page_size() {
                unsafe { self.alloc.deallocate(page_table_addr, PAGE_TABLE_LAYOUT) };
            }
        }

        unsafe {
            self.alloc
                .deallocate(self.root.cast::<u8>(), PAGE_DIRECTORY_LAYOUT)
        };
    }
}

/// Enable paging in the CPU.
///
/// # Safety
///
/// Valid page tables must have been previously loaded, and enabling paging
/// with those tables must not cause any existing pointers to refer to anything
/// they shouldn't.
pub unsafe fn enable() {
    #[bitfield(u32, default = 0)]
    struct CR0 {
        #[bit(16, rw)]
        write_protect: bool,
        #[bit(31, rw)]
        paging: bool,
    }

    const MASK: u32 = CR0::DEFAULT
        .with_write_protect(true)
        .with_paging(true)
        .raw_value();

    asm!(
        "
        mov {0}, cr0
        or {0}, {mask}
        mov cr0, {0}
        ",
        out(reg) _,
        mask = const MASK,
        options(nostack),
    );
}

lazy_static! {
    static ref PSE_SUPPORTED: bool = {
        #[bitfield(u32, default = 0)]
        struct EFlags {
            #[bit(21, rw)]
            id: bool,
        }

        let eflags_diff: u32;
        unsafe {
            asm!(
                "
        pushfd // Save original EFLAGS.

        // Get a copy of EFLAGS and modify it to toggle the mask bit.
        pushfd
        pop {0}
        xor {0}, {mask}
        push {0}

        popfd // Move the copy into EFLAGS.

        // Get a diff of what changed in EFLAGS after the attempted
        // modification.
        pushfd
        pop {0}
        xor {0}, [esp]

        popfd // Restore original EFLAGS.
                ",
                out(reg) eflags_diff,
                mask = const EFlags::DEFAULT.with_id(true).raw_value(),
            )
        };

        // If the attempted modification didn't change the id bit, then cpuid
        // isn't supported.
        if !EFlags::new_with_raw_value(eflags_diff).id() {
            return false;
        }

        #[bitfield(u32, default = 0)]
        struct CPUIDEdx {
            #[bit(3, rw)]
            pse: bool,
        }

        let core::arch::x86::CpuidResult { edx, .. } = unsafe { core::arch::x86::__cpuid(0) };
        CPUIDEdx::new_with_raw_value(edx).pse()
    };
    static ref PSE_ENABLED: bool = {
        // Check if PSE is already enabled (from the trapoline, if we're running
        // in the kernel).
        #[bitfield(u32, default = 0)]
        struct CR4 {
            #[bit(4, rw)]
            pse: bool,
        }

        let cr4: u32;
        unsafe { asm!("mov {}, cr4", out(reg) cr4, options(nomem, nostack)) };
        let cr4 = CR4::new_with_raw_value(cr4);
        if cr4.pse() {
            // If it is, early return true.
            return true;
        }

        if !*PSE_SUPPORTED {
            // If not, and it's not supported, early return false.
            return false;
        }

        // Otherwise, enable it and return true.
        unsafe { asm!("mov cr4, {}", in(reg) cr4.with_pse(true).raw_value() as usize, options(nostack)) };
        true
    };
}

pub struct MappingRange {
    pub phys_start: usize,
    pub virt_start: usize,
    pub len: usize,
    pub write: bool,
    pub user: bool,
}

pub fn kernel_mapping_ranges() -> [MappingRange; 5] {
    [
        MappingRange {
            phys_start: VIDEO_MEMORY_BASE,
            virt_start: VIDEO_MEMORY_BASE,
            len: VIDEO_MEMORY_SIZE.next_multiple_of(PAGE_FRAME_SIZE),
            write: true,
            user: false,
        },
        MappingRange {
            phys_start: kernel_start(),
            virt_start: virt::kernel_start(),
            len: kernel_data_start() - kernel_start(),
            write: false,
            user: false,
        },
        MappingRange {
            phys_start: kernel_data_start(),
            virt_start: virt::kernel_data_start(),
            len: kernel_end() - kernel_data_start(),
            write: true,
            user: false,
        },
        MappingRange {
            phys_start: kernel_end(),
            virt_start: virt::kernel_end(),
            len: main_stack_top() - kernel_end(),
            write: true,
            user: false,
        },
        MappingRange {
            phys_start: trampoline_heap_top(),
            virt_start: virt::trampoline_heap_top(),
            len: (usize::MAX - OFFSET - trampoline_heap_top()).next_multiple_of(PAGE_FRAME_SIZE),
            write: true,
            user: false,
        },
    ]
}
