use super::super::{ThreadControlBlock, Tid, IntrLevel};
use super::thread_manager::ThreadManager;
use crate::sync::intr::{intr_disable, intr_enable, intr_get_level};
use alloc::boxed::Box;
use core::arch::asm;
use core::ops::Index;

const ARRAY_REPEAT_VALUE: Option<Box<ThreadControlBlock>> = None;

pub struct ThreadManager128 {
    // list of threads being handled
    thread_list: [Option<Box<ThreadControlBlock>>; 128],

    // 4 x 32 = 128 TIDs maximum available
    pid_cache_1: u32,
    pid_cache_2: u32,
    pid_cache_3: u32,
    pid_cache_4: u32,
}

impl ThreadManager for ThreadManager128 {
    fn new() -> ThreadManager128 {
        ThreadManager128 {
            thread_list: [ARRAY_REPEAT_VALUE; 128],
            pid_cache_1: u32::MAX,
            pid_cache_2: u32::MAX,
            pid_cache_3: u32::MAX,
            pid_cache_4: u32::MAX,
        }
    }
    
    fn add(&mut self, thread:Box<ThreadControlBlock>) -> &Box<ThreadControlBlock> {
        intr_disable();
        let mut tid: Tid = 128;
        // TZCNT, LZCNT not available, thus treated as BSF -> bit of the first available 1
        // https://www.amd.com/content/dam/amd/en/documents/processor-tech-docs/programmer-references/24594.pdf#page=394
        // Need to dedicate ECX to pid, since CL used for shifting.
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
        (self.thread_list)
            .as_mut()[tid as usize] = Some(thread);
        let thread_ref: &Box<ThreadControlBlock> = 
            (self.thread_list)
                .index(tid as usize)
                .as_ref()
                .expect("Impossible.");
        intr_enable();
        thread_ref
    }

    // wrapper to allow box update post box consumption
    // assumes valid thread tid.
    fn add_existing(&mut self, thread:Box<ThreadControlBlock>) -> &Box<ThreadControlBlock> {
        assert!(intr_get_level() == IntrLevel::IntrOff);
        let tid: usize = thread.tid as usize;
        (self.thread_list)
            .as_mut()[tid] = Some(thread);
        let thread_ref: &Box<ThreadControlBlock> = 
            (self.thread_list)
                .index(tid)
                .as_ref()
                .expect("Impossible.");
        thread_ref
    }

    // NOTE: We assume tid valid.
    fn remove(&mut self, tid: Tid) -> Box<ThreadControlBlock> {
        intr_disable();
        let cache_num = tid / 32;
        let rel_ind = tid % 32;
        if cache_num == 0 {
            self.pid_cache_1 ^= 1 << rel_ind;
        }
        else if cache_num == 1 {
            self.pid_cache_2 ^= 1 << rel_ind;
        }
        else if cache_num == 2 {
            self.pid_cache_3 ^= 1 << rel_ind;
        }
        else {
            self.pid_cache_4 ^= 1 << rel_ind;
        }
        let thread: Box<ThreadControlBlock> = {
            <Option<Box<ThreadControlBlock>> as Clone>::clone(
                (self.thread_list)
                .index(tid as usize)).expect("Attempted removing unallocated TID.")
        };
        (self.thread_list)
            .as_mut()[tid as usize] = ARRAY_REPEAT_VALUE;
        intr_enable();
        thread
    }

    // NOTE: We assume tid valid.
    unsafe fn get_clone_ptr(&mut self, tid: Tid) -> *mut ThreadControlBlock {
        Box::into_raw(
            <Option<Box<ThreadControlBlock>> as Clone>::clone(
                (self.thread_list)
                .index(tid as usize)).expect("Attempted removing unallocated TID.")
        )
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
