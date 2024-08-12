#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]

use super::block::{BlockSector, BlockType, BlockDevice };
use alloc::boxed::Box;

pub struct Partition {
    block: Box<dyn BlockDevice>,
    start: BlockSector, 
}

impl Partition {

    pub fn partition_scan(){

    }


}