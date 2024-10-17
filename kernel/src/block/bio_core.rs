use crate::block::bio_request::BioReq;
use crate::block::block_error::BlockError;

pub trait BioScheduler {
    /// Add a request to the scheduler.
    fn enqueue(&mut self, r: &mut BioReq);

    /// Wait for the next request to be completed.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it provides no checks on anything. The caller must verify
    /// the constraints of the request.
    unsafe fn wait(&mut self) -> Result<(), BlockError>;
}
