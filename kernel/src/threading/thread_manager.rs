use super::{ThreadControlBlock, Tid};
use alloc::boxed::Box;
use core::arch::asm;

pub static mut THREADMANAGER: Option<Box<dyn ThreadManager>> = None;

trait ThreadManager {
    fn new() -> Self
    where 
        Self: Sized;
    fn allocate_tid(self: &mut Self) -> Tid;
    fn deallocate_tid(self: &mut Self,  thread: &Box<ThreadControlBlock>) -> ();
}

pub fn initialize_thread_manager() {
    unsafe {
        THREADMANAGER = Some(Box::new(ThreadManager128::new()));
    }
}

// #[derive(Copy, Clone)]
pub struct ThreadManager128 {
    // list of threads being handled
    // pub thread_list: [Box<Option<ThreadControlBlock>>; 128],

    // 4 x 32 = 128 TIDs maximum available
    pid_cache_1: u32,
    pid_cache_2: u32,
    pid_cache_3: u32,
    pid_cache_4: u32,
}

impl ThreadManager for ThreadManager128 {
    fn new() -> ThreadManager128 {
        ThreadManager128 {
            // thread_list: [Box::new(None); 128],
            pid_cache_1: u32::MAX,
            pid_cache_2: u32::MAX,
            pid_cache_3: u32::MAX,
            pid_cache_4: u32::MAX,
        }
    }

    // TZCNT, LZCNT not available, thus treated as BSF -> bit of the first available 1
    // https://www.amd.com/content/dam/amd/en/documents/processor-tech-docs/programmer-references/24594.pdf#page=394
    // Need to dedicate ECX to pid, since CL used for shifting.
    fn allocate_tid(self: &mut Self) -> Tid {
        let mut tid: Tid = 128;
        unsafe {
            asm!(
                "
            mov {msk}, 1
            bsf ecx, {c1}
            cmp ecx, 32
            jl $1f
            bsf ecx, {c2}
            cmp ecx, 32
            jl $2f
            bsf ecx, {c3}
            cmp ecx, 32
            jl $3f
            bsf ecx, {c4}
            cmp ecx, 32
            jl $4f
            jmp $5f
        1:
            shl {msk}, cl
            xor {c1}, {msk}
            jmp $5f
        2:
            shl {msk}, cl
            xor {c2}, {msk}
            add ecx, 32
            jmp $5f
        3:
            shl {msk}, cl
            xor {c3}, {msk}
            add ecx, 64
            jmp $5f
        4:
            shl {msk}, cl
            xor {c4}, {msk}
            add ecx, 96
        5:
                ",
                c1 = inout(reg) self.pid_cache_1,
                c2 = inout(reg) self.pid_cache_2,
                c3 = inout(reg) self.pid_cache_3,
                c4 = inout(reg) self.pid_cache_4,
                inout("ecx") tid,
                msk = out(reg) _,
            );
        }
        if tid > 127{
            panic!("No PID available!");
        }
        tid
    }

    fn deallocate_tid(self: &mut Self, thread: &Box<ThreadControlBlock>) -> () {
        // let tid = Box::into_inner(thread).tid;
        let tid = 0;
        let cache_num = tid / 32;
        let rel_ind = tid % 32;
        if cache_num == 0 {
            self.pid_cache_1 = self.pid_cache_1 ^ (1 << rel_ind);
        }
        else if cache_num == 1 {
            self.pid_cache_2 = self.pid_cache_2 ^ (1 << rel_ind);
        }
        else if cache_num == 2 {
            self.pid_cache_3 = self.pid_cache_3 ^ (1 << rel_ind);
        }
        else {
            self.pid_cache_4 = self.pid_cache_4 ^ (1 << rel_ind);
        }

        // is this call responsible ?
        // Box::into_inner(thread).tid = -1
    }
}


/*

let mut cache1: u32 = 0;
    let mut cache2: u32 = 0;
    let mut cache3: u32 = 3;
    let mut cache4: u32 = 0;
    let mut pid: u32 = 128;
    let mut msk: u32 = 128;
    // bit of the first available pid
    // 
    // 
    unsafe {
        asm!(
            "
        mov {msk}, 1
        bsf ecx, {c1}
        cmp ecx, 32
        jl $1f
        bsf ecx, {c2}
        cmp ecx, 32
        jl $2f
        bsf ecx, {c3}
        cmp ecx, 32
        jl $3f
        bsf ecx, {c4}
        cmp ecx, 32
        jl $4f
        jmp $5f
    1:
        shl {msk}, cl
        xor {c1}, {msk}
        jmp $5f
    2:
        shl {msk}, cl
        xor {c2}, {msk}
        add ecx, 32
        jmp $5f
    3:
        shl {msk}, cl
        xor {c3}, {msk}
        add ecx, 64
        jmp $5f
    4:
        shl {msk}, cl
        xor {c4}, {msk}
        add ecx, 96
    5:
            ",
            c1 = inout(reg) cache1,
            c2 = inout(reg) cache2,
            c3 = inout(reg) cache3,
            c4 = inout(reg) cache4,
            inout("ecx") pid,
            msk = out(reg) _,
        );
    }
    if pid > 127{
        panic!("No PID available!");
    }
    println!("{} {} {} {} {}", cache1, cache2, cache3, cache4, pid);
    assert!(0 < 0);
*/