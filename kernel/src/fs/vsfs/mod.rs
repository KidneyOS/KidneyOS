use vsfs::VSFS;

pub mod vsfs;

#[cfg(test)]
mod test {
    use super::*;
    use core::fmt::Error;
    use std::fs::File;
    use std::env;
    use std::io::Read;
    use crate::block;
    use crate::block::block_core::test::block_from_file;
    use crate::fs::vsfs::vsfs::VSFS_BLOCK_SIZE;

    fn open_disk_image(path: &str) -> Result<VSFS, Error> {
        // Print current directory
        println!("Current directory: {:?}", env::current_dir().unwrap());
        
        // Print file size and first few bytes for debugging
        let file = File::open(path).unwrap();
        let metadata = file.metadata().unwrap();
        println!("File size: {} bytes", metadata.len());

        // Read and print first few bytes to check format
        let mut buffer = vec![0; VSFS_BLOCK_SIZE];  // Read first 4096 bytes
        let mut file_clone = file.try_clone().unwrap();
        file_clone.read_exact(&mut buffer).unwrap();
        //println!("First 64 bytes: {:?}", &buffer);

        // // Compare endianness for each field
        // println!("\nComparing endianness of fields:");
        
        // // Magic number (first 8 bytes)
        // let magic_le = u64::from_le_bytes(buffer[0..8].try_into().unwrap());
        // let magic_be = u64::from_be_bytes(buffer[0..8].try_into().unwrap());
        // println!("Magic number - LE: {:#x}, BE: {:#x}", magic_le, magic_be);

        // // total_inodes (bytes 8-12)
        // let inodes_le = u64::from_le_bytes(buffer[8..16].try_into().unwrap());
        // let inodes_be = u64::from_be_bytes(buffer[8..16].try_into().unwrap());
        // println!("File system size in bytes - LE: {}, BE: {}", inodes_le, inodes_be);

        // // total_blocks (bytes 12-16)
        // let blocks_le = u32::from_le_bytes(buffer[16..20].try_into().unwrap());
        // let blocks_be = u32::from_be_bytes(buffer[16..20].try_into().unwrap());
        // println!("Total number of inodes (set by mkfs) - LE: {}, BE: {}", blocks_le, blocks_be);

        // // inode_bitmap_block (bytes 16-20)
        // let inode_bitmap_le = u32::from_le_bytes(buffer[20..24].try_into().unwrap());
        // let inode_bitmap_be = u32::from_be_bytes(buffer[20..24].try_into().unwrap());
        // println!("Number of available inodes - LE: {}, BE: {}", inode_bitmap_le, inode_bitmap_be);

        // // data_bitmap_block (bytes 20-24)
        // let data_bitmap_le = u32::from_le_bytes(buffer[24..28].try_into().unwrap());
        // let data_bitmap_be = u32::from_be_bytes(buffer[24..28].try_into().unwrap());
        // println!("File system size in blocks - LE: {}, BE: {}", data_bitmap_le, data_bitmap_be);

        // // inode_table_block (bytes 24-28)
        // let inode_table_le = u32::from_le_bytes(buffer[28..32].try_into().unwrap());
        // let inode_table_be = u32::from_be_bytes(buffer[28..32].try_into().unwrap());
        // println!("Number of available blocks in file sys - LE: {}, BE: {}", inode_table_le, inode_table_be);

        // // data_block_start (bytes 28-32)
        // let data_start_le = u32::from_le_bytes(buffer[32..36].try_into().unwrap());
        // let data_start_be = u32::from_be_bytes(buffer[32..36].try_into().unwrap());
        // println!("First block after inode table - LE: {}, BE: {}", data_start_le, data_start_be);


        // Read and print first few bytes to check format
        // let mut buffer = vec![0; 128];  // Read first 64 bytes
        // let mut file_clone = file.try_clone().unwrap();
        // file_clone.read_exact(&mut buffer).unwrap();
        // println!("First 128 bytes: {:?}", &buffer);

        // Try to create block device
        let block = block_from_file(file);
        let vsfs = VSFS::new(block).unwrap();
        println!("Successfully created VSFS");
        // print superblock's every field
        println!("Magic number: {:#x}", vsfs.superblock.magic_number);
        println!("File system size in bytes: {}", vsfs.superblock.fs_size);
        println!("Total number of inodes (set by mkfs): {}", vsfs.superblock.num_inodes);
        println!("Number of available inodes: {}", vsfs.superblock.free_inodes);
        println!("File system size in blocks: {}", vsfs.superblock.num_blocks);
        println!("Number of available blocks: {}", vsfs.superblock.free_blocks);
        println!("First block after inode table: {}", vsfs.superblock.data_start);
        
        // Return error always for now
        Err(Error)
        // Try to create VSFS
        // match VSFS::new(block) {
        //     Ok(vsfs) => {
        //         println!("Successfully created VSFS");
        //         // print superblock's every field
        //         println!("Magic number: {:#x}", vsfs.superblock.magic_number);
        //         println!("File system size in bytes: {}", vsfs.superblock.fs_size);
        //         println!("Total number of inodes (set by mkfs): {}", vsfs.superblock.num_inodes);
        //         println!("Number of available inodes: {}", vsfs.superblock.free_inodes);
        //         println!("File system size in blocks: {}", vsfs.superblock.num_blocks);
        //         println!("Number of available blocks: {}", vsfs.superblock.free_blocks);
        //         println!("First block after inode table: {}", vsfs.superblock.data_start);

        //         Ok(vsfs)
        //     },
        //     Err(e) => {
        //         println!("Failed to create VSFS: {:?}", e);
        //         panic!("VSFS creation failed");
        //     }
        // }
    }

    #[test]
    fn test_1file() {
        let image_path = "src/fs/tests/vsfs/images/vsfs-1file.disk";
        
        // Check if file exists
        if !std::path::Path::new(image_path).exists() {
            panic!("Test image file not found at path: {}", image_path);
        }
        
        let vsfs = open_disk_image(image_path);
        // Rest of test...

        // Panic
        panic!("Test failed");
    }

    // #[test]
    // fn test_3files() {
    //     let image_path = "src/fs/tests/vsfs/images/vsfs-3file.disk";
        
    //     // Check if file exists
    //     if !std::path::Path::new(image_path).exists() {
    //         panic!("Test image file not found at path: {}", image_path);
    //     }
        
    //     let vsfs = open_disk_image(image_path);
    //     // Rest of test...
    // }

    // #[test]
    // fn test_empty() {
    //     let image_path = "src/fs/tests/vsfs/images/vsfs-empty.disk";
        
    //     // Check if file exists
    //     if !std::path::Path::new(image_path).exists() {
    //         panic!("Test image file not found at path: {}", image_path);
    //     }
        
    //     let vsfs = open_disk_image(image_path);
    //     // Rest of test...
    // }

    // #[test]
    // fn test_42files() {
    //     let image_path = "src/fs/tests/vsfs/images/vsfs-42file.disk";
        
    //     // Check if file exists
    //     if !std::path::Path::new(image_path).exists() {
    //         panic!("Test image file not found at path: {}", image_path);
    //     }
        
    //     let vsfs = open_disk_image(image_path);
    //     // Rest of test...
    // }
}