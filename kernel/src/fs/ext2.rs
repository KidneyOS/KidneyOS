

#[repr(C, packed)]
struct EXT2_Superblock {
    inode_count: u32,
    block_count: u32,
    su_blocks: u32,
    unallocated_blocks: u32,

}



