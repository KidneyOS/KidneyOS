use crate::block::bio_core::BioScheduler;
use crate::block::bio_request::{BioOp, BioReq};
use crate::block::block_core::BLOCK_MANAGER;
use crate::block::block_error::BlockError;
use alloc::vec::Vec;

/// A NOOP scheduler that contains no scheduling logic.
pub struct NoopScheduler {
    block: usize,
    queue: Vec<BioReq>,
}

impl NoopScheduler {
    pub fn new(block: usize) -> Self {
        Self {
            block,
            queue: Vec::new(),
        }
    }
}

impl BioScheduler for NoopScheduler {
    fn enqueue(&mut self, r: &mut BioReq) {
        self.queue.push(r.clone());
    }

    unsafe fn wait(&mut self) -> Result<(), BlockError> {
        let r = self.queue.pop().unwrap();

        let block = BLOCK_MANAGER.by_id(self.block).unwrap();

        // Perform the operation
        if r.op == BioOp::BioRead {
            unsafe { block.read_raw(r.sector, core::slice::from_raw_parts_mut(r.buffer, 512)) }
        } else {
            unsafe { block.write_raw(r.sector, core::slice::from_raw_parts(r.buffer, 512)) }
        }
    }
}
