use core::ptr;
use core::arch::asm;
const PGSIZE: usize = 4096; // Page size is typically 4 KiB.


/* Creates a new page directory that has mappings for kernel
   virtual addresses, but none for user virtual addresses.
   Returns the new page directory, or a null pointer if memory
   allocation fails. */
fn pagedir_create() -> Option<*mut u32> {
    unsafe {
        // Allocate a page of memory.
        let pd = palloc_get_page(0);
        if !pd.is_null() {
            // Copy the initial page directory into the new page directory.
            ptr::copy_nonoverlapping(init_page_dir, pd, PGSIZE / 4); // Divide by 4 because we're copying u32 values.
        }
        // In Rust, we generally use Option to represent the possibility of null pointers.
        pd.as_mut().map(|p| p as *mut u32)
    }
}

/* Loads page directory PD into the CPU's page directory base
   register. */
pub unsafe fn pagedir_activate(pd: *mut u32) {
    let pd = if pd.is_null() { init_page_dir } else { pd };

    // Inline assembly to move the physical address of the page directory into the CR3 register.
    // This is a privileged operation and might not be allowed depending on your operating system and environment.
    asm!(
        "mov cr3, {}",
        in(reg) vtop(pd), //TODO: vtop() Returns physical address at which kernel virtual address VADDR is mapped.
        options(nostack, nomem)
    );
}

/* Destroys page directory PD, freeing all the pages it
   references. */
pub unsafe fn pagedir_destroy(mut pd: *mut u32) {
    if pd.is_null() {
        return;
    }

    assert!(pd != init_page_dir as *mut u32, "Cannot destroy initial page directory.");

    let pd_end = pd.add(pd_no(PHYS_BASE));
    while pd < pd_end {
        if *pd & PTE_P != 0 {
            let mut pt = pde_get_pt(*pd);
            let pt_end = pt.add(PGSIZE / std::mem::size_of::<u32>());
            while pt < pt_end {
                if *pt & PTE_P != 0 {
                    palloc_free_page(pte_get_page(*pt));
                }
                pt = pt.add(1);
            }
            palloc_free_page(pt);
        }
        pd = pd.add(1);
    }
    palloc_free_page(pd);
}