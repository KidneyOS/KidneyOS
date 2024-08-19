use super::sync::irq::MutexIrq;

pub struct SemaphoreIrq {
    count: Mutex<u32>
}
    
