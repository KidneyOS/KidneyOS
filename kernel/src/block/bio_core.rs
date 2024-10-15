use crate::block::bio_request::BioReq;
use crate::block::block_error::BlockError;

pub trait BioScheduler {
    /// Add a request to the scheduler.
    fn enqueue(&mut self, r: &mut BioReq);

    /// Wait for the next request to be completed.
    fn wait(&mut self) -> Result<(), BlockError>;
}
