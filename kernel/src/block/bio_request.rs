#![allow(dead_code)] // Suppress unused warnings

use crate::block::block_core::BlockSector;

pub type OnBioCompletion = fn(BioReq) -> ();

pub enum BioOp {
    BioRead,
    BioWrite,
}

pub struct BioReq {
    sector: BlockSector,
    buffer: *mut u8,
    op: BioOp,
    ddl: u64,
    priority: u8,
    on_complete: Option<OnBioCompletion>,
}

impl BioReq {
    pub fn new(
        sector: BlockSector,
        buffer: *mut u8,
        op: BioOp,
        ddl: u64,
        priority: u8,
        on_complete: Option<OnBioCompletion>,
    ) -> Self {
        Self {
            sector,
            buffer,
            op,
            ddl,
            priority,
            on_complete,
        }
    }
}
