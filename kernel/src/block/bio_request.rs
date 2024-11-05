#![allow(dead_code)] // Suppress unused warnings

use crate::block::block_core::BlockSector;

pub type OnBioCompletion = fn(BioReq) -> ();

#[derive(Clone, Eq, PartialEq)]
pub enum BioOp {
    BioRead,
    BioWrite,
}

#[derive(Clone)]
pub struct BioReq {
    pub(crate) sector: BlockSector,
    pub(crate) buffer: *mut u8,
    pub(crate) op: BioOp,
    pub(crate) ddl: u64,
    pub(crate) priority: u8,
    pub(crate) on_complete: Option<OnBioCompletion>,
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
