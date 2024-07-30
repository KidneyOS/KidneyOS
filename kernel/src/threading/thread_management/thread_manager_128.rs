use super::super::{ThreadControlBlock, Tid};
use super::thread_manager::ThreadManager;
use core::arch::asm;
use core::ops::IndexMut;

const ARRAY_REPEAT_VALUE: Option<ThreadControlBlock> = None;

pub struct ThreadManager128 {
    // list of threads being handled
    thread_list: [Option<ThreadControlBlock>; 128],

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
    
    // NOTE: We assume interrupts disabled
    fn add(&mut self, mut thread:ThreadControlBlock) -> Tid {
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
        thread.tid = tid;
        (self.thread_list)
            .as_mut()[tid as usize] = Some(thread);
        tid
    }

    // NOTE: We assume tid valid, interrupts disabled
    fn remove(&mut self, tid: Tid) -> ThreadControlBlock {
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
        let thread: ThreadControlBlock = 
            (self.thread_list)
                .index_mut(tid as usize)
                .take()
                .expect("Invalid Tid, thread doesn't exist");
        thread
    }

    fn get(&mut self, tid: Tid) -> ThreadControlBlock {
        (self.thread_list)
            .index_mut(tid as usize)
            .take()
            .expect("Invalid Tid, thread doesn't exist")
    }

    fn set(&mut self, thread: ThreadControlBlock) -> Tid {
        let tid = thread.tid;
        (self.thread_list)[tid as usize] =  Some(thread);
        tid
    }
}
