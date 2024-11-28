use crate::block::block_core::{Block, BLOCK_SECTOR_SIZE};
use crate::vfs::{
    DirEntries, Error, FileInfo, INodeNum, INodeType, Path, RawDirEntry, Result, SimpleFileSystem,
};
use alloc::{string::String, vec, vec::Vec};
use core::cmp::{max, min};
use zerocopy::{FromBytes, FromZeroes};
#[allow(clippy::module_inception)]
pub mod vsfs;
use vsfs::{Bitmap, SuperBlock};

pub const VSFS_BLOCK_SIZE: usize = 4096; // same block size in bytes as the vsfs disk images provided
pub const BLOCK_SIZE_RATIO: usize = VSFS_BLOCK_SIZE / BLOCK_SECTOR_SIZE; // assume that the block size is a multiple of the sector size
pub const VSFS_MAGIC: u64 = 0xC5C369A4C5C369A4; // same magic number from the vsfs disk images
pub const VSFS_DIRECT_BLOCKS: usize = 5; // same number of direct blocks as the vsfs disk images

/* vsfs has simple layout
 *   Block 0: superblock
 *   Block 1: inode bitmap
 *   Block 2: data bitmap
 *   Block 3: start of inode table
 *   First data block after inode table
 */
pub const VSFS_SUPERBLOCK_BLOCK: u32 = 0;
pub const VSFS_INODE_BITMAP_BLOCK: u32 = 1;
pub const VSFS_DATA_BITMAP_BLOCK: u32 = 2;
pub const VSFS_INODE_TABLE_BLOCK: u32 = 3;

pub const VSFS_INODE_SIZE: usize = 64; // same inode size as the vsfs disk images

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, FromZeroes)]
pub struct Timespec {
    tv_sec: i64,  // seconds since the Epoch
    tv_nsec: i64, // nanoseconds
}

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, FromZeroes)]
pub struct Inode {
    mode: u32,                                // File type and permissions.
    n_links: u32,                             // Number of hard links.
    block_count: u32,                         // Number of blocks in the file.
    _padding: u32,                            // Unused padding to fill out 4 bytes.
    size: u64,                                // File size in bytes.
    mtime: Timespec,                          // Last modification time.
    direct_blocks: [u32; VSFS_DIRECT_BLOCKS], // Direct block pointers.
    indirect_block: u32,                      // Indirect block pointer.
}

// Define the VSFS struct that will hold the superblock, bitmaps, and data blocks
pub struct VSFS {
    pub superblock: SuperBlock,
    pub inode_bitmap: Bitmap,
    pub data_bitmap: Bitmap,
    pub inodes: Vec<Inode>,
    block: Block,
    root_inode: INodeNum,
}

impl VSFS {
    pub fn new(block: Block) -> Result<Self> {
        // Read the superblock from the first block
        let mut superblock = SuperBlock {
            magic_number: 0,
            fs_size: 0,
            num_inodes: 0,
            free_inodes: 0,
            num_blocks: 0,
            free_blocks: 0,
            data_start: 0,
        };

        let mut first_sector = [0; 512];
        block.read(0, &mut first_sector)?;

        // Parse the superblock from the first sector
        superblock.magic_number = u64::from_le_bytes(first_sector[0..8].try_into().unwrap());

        // Check if the magic number matches
        if superblock.magic_number != VSFS_MAGIC {
            return Err(Error::Unsupported);
        }

        superblock.fs_size = u64::from_le_bytes(first_sector[8..16].try_into().unwrap());
        superblock.num_inodes = u32::from_le_bytes(first_sector[16..20].try_into().unwrap());
        superblock.free_inodes = u32::from_le_bytes(first_sector[20..24].try_into().unwrap());
        superblock.num_blocks = u32::from_le_bytes(first_sector[24..28].try_into().unwrap());
        superblock.free_blocks = u32::from_le_bytes(first_sector[28..32].try_into().unwrap());
        superblock.data_start = u32::from_le_bytes(first_sector[32..36].try_into().unwrap());

        let mut data_blocks = Vec::new();

        for i in superblock.data_start..superblock.num_blocks {
            let mut data = vec![0; VSFS_BLOCK_SIZE];
            for j in 0..BLOCK_SIZE_RATIO {
                block.read(
                    j as u32 + i * BLOCK_SIZE_RATIO as u32,
                    &mut data[(j * BLOCK_SECTOR_SIZE)..(j * BLOCK_SECTOR_SIZE + BLOCK_SECTOR_SIZE)],
                )?;
            }
            data_blocks.push(data);
        }

        // Read the inode bitmap
        let mut inode_bitmap = Bitmap::new(superblock.num_inodes);
        let mut bits = vec![0; VSFS_BLOCK_SIZE];
        for i in 0..BLOCK_SIZE_RATIO {
            let index = i + (VSFS_INODE_BITMAP_BLOCK as usize * BLOCK_SIZE_RATIO);
            block.read(
                index as u32,
                &mut bits[(i * BLOCK_SECTOR_SIZE)..(i * BLOCK_SECTOR_SIZE + BLOCK_SECTOR_SIZE)],
            )?;
        }
        inode_bitmap.bits = bits;

        // Read the data bitmap
        let mut data_bitmap = Bitmap::new(superblock.num_blocks);
        let mut bits = vec![0; VSFS_BLOCK_SIZE];
        for i in 0..BLOCK_SIZE_RATIO {
            block.read(
                (i + (VSFS_DATA_BITMAP_BLOCK as usize * BLOCK_SIZE_RATIO)) as u32,
                &mut bits[(i * BLOCK_SECTOR_SIZE)..(i * BLOCK_SECTOR_SIZE + BLOCK_SECTOR_SIZE)],
            )?;
        }
        data_bitmap.bits = bits;

        // Create the root inode (default to 0)
        let root_inode = 0;

        let inode_ratio = VSFS_BLOCK_SIZE / VSFS_INODE_SIZE;

        // Read the inodes and store them in a vector
        let mut inodes = Vec::new();
        for i in VSFS_INODE_TABLE_BLOCK..superblock.data_start {
            let mut buffer = vec![0; VSFS_BLOCK_SIZE];
            for j in 0..BLOCK_SIZE_RATIO {
                block.read(
                    (j + (i as usize * BLOCK_SIZE_RATIO)) as u32,
                    &mut buffer
                        [(j * BLOCK_SECTOR_SIZE)..(j * BLOCK_SECTOR_SIZE + BLOCK_SECTOR_SIZE)],
                )?;
            }

            for k in 0..(inode_ratio) {
                let mut inode = Inode {
                    mode: 0,
                    n_links: 0,
                    block_count: 0,
                    size: 0,
                    mtime: Timespec {
                        tv_sec: 0,
                        tv_nsec: 0,
                    },
                    direct_blocks: [0; VSFS_DIRECT_BLOCKS],
                    indirect_block: 0,
                    _padding: 0,
                };

                if inode_bitmap
                    .is_allocated((i - VSFS_INODE_TABLE_BLOCK) * inode_ratio as u32 + k as u32)
                {
                    inode =
                        Inode::read_from(&buffer[k * VSFS_INODE_SIZE..(k + 1) * VSFS_INODE_SIZE])
                            .unwrap();
                    // inode.mode = u32::from_le_bytes(
                    //     buffer[k * VSFS_INODE_SIZE..k * VSFS_INODE_SIZE + 4]
                    //         .try_into()
                    //         .unwrap(),
                    // );
                    // inode.n_links = u32::from_le_bytes(
                    //     buffer[k * VSFS_INODE_SIZE + 4..k * VSFS_INODE_SIZE + 8]
                    //         .try_into()
                    //         .unwrap(),
                    // );
                    // inode.block_count = u32::from_le_bytes(
                    //     buffer[k * VSFS_INODE_SIZE + 8..k * VSFS_INODE_SIZE + 12]
                    //         .try_into()
                    //         .unwrap(),
                    // );
                    // inode._padding = u32::from_le_bytes(
                    //     buffer[k * VSFS_INODE_SIZE + 12..k * VSFS_INODE_SIZE + 16]
                    //         .try_into()
                    //         .unwrap(),
                    // );
                    // inode.size = u64::from_le_bytes(
                    //     buffer[k * VSFS_INODE_SIZE + 16..k * VSFS_INODE_SIZE + 24]
                    //         .try_into()
                    //         .unwrap(),
                    // );
                    // inode.mtime.tv_sec = i64::from_le_bytes(
                    //     buffer[k * VSFS_INODE_SIZE + 24..k * VSFS_INODE_SIZE + 32]
                    //         .try_into()
                    //         .unwrap(),
                    // );
                    // inode.mtime.tv_nsec = i64::from_le_bytes(
                    //     buffer[k * VSFS_INODE_SIZE + 32..k * VSFS_INODE_SIZE + 40]
                    //         .try_into()
                    //         .unwrap(),
                    // );
                    // inode.direct_blocks = [
                    //     u32::from_le_bytes(
                    //         buffer[k * VSFS_INODE_SIZE + 40..k * VSFS_INODE_SIZE + 44]
                    //             .try_into()
                    //             .unwrap(),
                    //     ),
                    //     u32::from_le_bytes(
                    //         buffer[k * VSFS_INODE_SIZE + 44..k * VSFS_INODE_SIZE + 48]
                    //             .try_into()
                    //             .unwrap(),
                    //     ),
                    //     u32::from_le_bytes(
                    //         buffer[k * VSFS_INODE_SIZE + 48..k * VSFS_INODE_SIZE + 52]
                    //             .try_into()
                    //             .unwrap(),
                    //     ),
                    //     u32::from_le_bytes(
                    //         buffer[k * VSFS_INODE_SIZE + 52..k * VSFS_INODE_SIZE + 56]
                    //             .try_into()
                    //             .unwrap(),
                    //     ),
                    //     u32::from_le_bytes(
                    //         buffer[k * VSFS_INODE_SIZE + 56..k * VSFS_INODE_SIZE + 60]
                    //             .try_into()
                    //             .unwrap(),
                    //     ),
                    // ];
                    // inode.indirect_block = u32::from_le_bytes(
                    //     buffer[k * VSFS_INODE_SIZE + 60..k * VSFS_INODE_SIZE + 64]
                    //         .try_into()
                    //         .unwrap(),
                    // );
                }

                inodes.push(inode);
            }
        }

        Ok(Self {
            superblock,
            inode_bitmap,
            data_bitmap,
            inodes,
            // data_blocks,
            block,
            root_inode,
        })
    }
}

impl SimpleFileSystem for VSFS {
    fn root(&self) -> INodeNum {
        self.root_inode
    }

    fn open(&mut self, inode: INodeNum) -> Result<()> {
        if self.inodes[inode as usize].mode != 16895 {
            return Err(Error::NotDirectory);
        } else if !self.inode_bitmap.is_allocated(inode) {
            return Err(Error::NotFound);
        }
        Ok(())
    }

    // Read the directory entries for the given inode
    fn readdir(&mut self, dir: INodeNum) -> Result<DirEntries> {
        // Read the inode from the inodes vector
        let inode = self.inodes[dir as usize];
        let mut entries = Vec::new();
        let mut names = String::new();

        // TODO: test this && support indirect block
        // For now, assume that the directory entries are stored in the direct blocks
        let num_blocks = inode.block_count;
        let mut data = vec![0; VSFS_BLOCK_SIZE * num_blocks as usize];

        // First read all direct blocks
        for i in 0..min(VSFS_DIRECT_BLOCKS, num_blocks as usize) {
            for j in 0..BLOCK_SIZE_RATIO {
                self.block.read(
                    j as u32 + inode.direct_blocks[i] * BLOCK_SIZE_RATIO as u32,
                    &mut data[(j * BLOCK_SECTOR_SIZE + i * VSFS_BLOCK_SIZE)
                        ..(j * BLOCK_SECTOR_SIZE + BLOCK_SECTOR_SIZE + i * VSFS_BLOCK_SIZE)],
                )?;
            }
        }
        // Then read the indirect block

        let mut db_index = 0;
        while db_index < inode.block_count * VSFS_BLOCK_SIZE as u32 {
            let slice = &data[db_index as usize..(db_index + 256) as usize];

            let inode_num = u32::from_le_bytes(slice[0..4].try_into().unwrap());
            if inode_num < 0x8000 {
                // First get the file name
                let file_name = &slice[4..];
                let name_index = names.len();

                let file_name_bytes = file_name
                    .iter()
                    .copied()
                    .take_while(|&byte| byte != 0)
                    .collect();
                let mut file_name_str = String::from_utf8(file_name_bytes)
                    .map_err(|_| Error::IO("bad UTF-8 in file name".into()))?;
                file_name_str.push('\0');
                names.push_str(&file_name_str);

                // Then get the entry
                let entry = RawDirEntry {
                    inode: inode_num,
                    name: name_index,
                    r#type: if self.inodes[inode_num as usize].mode == 33152 {
                        INodeType::File
                    } else {
                        INodeType::Directory
                    },
                };

                entries.push(entry);
            }

            db_index += 256;
        }

        Ok(DirEntries {
            filenames: names,
            entries,
        })
    }

    fn release(&mut self, _inode: INodeNum) {
        todo!()
    }

    fn read(&mut self, file: INodeNum, offset: u64, buf: &mut [u8]) -> Result<usize> {
        // Assume offset is a multiple of the sector size
        // TODO: implement offset && correct read size && handle odd buf size
        // Read the inode from the inodes vector
        let inode = self.inodes[file as usize];
        let file_size = inode.size as usize;
        let read_size = file_size - offset as usize; // How many bytes to read
        let buf_size = buf.len(); // Size of the buffer
        if read_size as isize <= 0 {
            return Ok(0);
        }

        let read_start_block: usize = (offset / VSFS_BLOCK_SIZE as u64) as usize;
        let read_start_offset = offset % VSFS_BLOCK_SIZE as u64;
        let read_start_sector = read_start_offset / BLOCK_SECTOR_SIZE as u64;
        // println!("Read start block: {}", read_start_block);
        // println!("Read start offset: {}", read_start_offset);
        // println!("Read start sector: {}", read_start_sector);

        // println!("Read size: {}", read_size);
        // println!("File size: {}", file_size);

        let mut bytes_read = 0;

        let num_blocks = inode.block_count;

        // First read all direct blocks
        for i in read_start_block..min(VSFS_DIRECT_BLOCKS, num_blocks as usize) {
            let j_start = if i == read_start_block {
                read_start_sector as usize
            } else {
                0
            };
            for j in j_start..BLOCK_SIZE_RATIO {
                self.block.read(
                    j as u32 + inode.direct_blocks[i] * BLOCK_SIZE_RATIO as u32,
                    &mut buf[bytes_read..bytes_read + BLOCK_SECTOR_SIZE],
                )?;
                bytes_read += BLOCK_SECTOR_SIZE;
                if buf_size - bytes_read == 0 {
                    return Ok(bytes_read);
                }
            }
        }
        // Then read the indirect block if needed
        if num_blocks > VSFS_DIRECT_BLOCKS as u32 && inode.indirect_block != 0 {
            // Read the indirect block
            let mut indirect_data = vec![0; VSFS_BLOCK_SIZE];
            for i in 0..BLOCK_SIZE_RATIO {
                self.block.read(
                    i as u32 + inode.indirect_block * BLOCK_SIZE_RATIO as u32,
                    &mut indirect_data
                        [(i * BLOCK_SECTOR_SIZE)..(i * BLOCK_SECTOR_SIZE + BLOCK_SECTOR_SIZE)],
                )?;
            }

            // Iterate through the indirect block. every 8 bytes is a data block number. Store the data block number in a vector
            let mut indirect_blocks = Vec::new();
            for i in 0..indirect_data.len() / 8 {
                let data_block =
                    u32::from_le_bytes(indirect_data[(i * 4)..(i * 4 + 4)].try_into().unwrap());
                if data_block != 0 {
                    indirect_blocks.push(data_block);
                }
            }

            // Read the indirect data blocks
            let mut index = VSFS_DIRECT_BLOCKS;
            #[allow(clippy::needless_range_loop)]
            for i in max(0, read_start_block as isize - VSFS_DIRECT_BLOCKS as isize) as usize
                ..indirect_blocks.len()
            {
                let j_start = if index == read_start_block {
                    read_start_sector as usize
                } else {
                    0
                };
                for j in j_start..BLOCK_SIZE_RATIO {
                    self.block.read(
                        j as u32 + indirect_blocks[i] * BLOCK_SIZE_RATIO as u32,
                        &mut buf[bytes_read..bytes_read + BLOCK_SECTOR_SIZE],
                    )?;
                    bytes_read += BLOCK_SECTOR_SIZE;
                    if buf_size - bytes_read == 0 {
                        return Ok(bytes_read);
                    }
                }
                index += 1;
            }
        }

        // self.block.read(inode.direct_blocks[0] * BLOCK_SIZE_RATIO as u32, &mut sector_data)?;

        // Read # of bytes equal to the minimum of:
        //   - the buffer size
        //   - the amount of bytes left in the file
        //   - the entire sector (starting from sector_offset)

        // buf[..bytes_read].copy_from_slice(
        //     &sector_data[offset as usize..(offset + bytes_read as u64) as usize]
        // );

        Ok(bytes_read)
    }

    fn stat(&mut self, _file: INodeNum) -> Result<FileInfo> {
        todo!()
    }

    fn readlink(&mut self, _link: INodeNum) -> Result<String> {
        todo!()
    }

    fn create(&mut self, _parent: INodeNum, _name: &Path) -> Result<INodeNum> {
        Err(Error::ReadOnlyFS)
    }

    fn mkdir(&mut self, _parent: INodeNum, _name: &Path) -> Result<INodeNum> {
        Err(Error::ReadOnlyFS)
    }

    fn unlink(&mut self, _parent: INodeNum, _name: &Path) -> Result<()> {
        Err(Error::ReadOnlyFS)
    }

    fn rmdir(&mut self, _parent: INodeNum, _name: &Path) -> Result<()> {
        Err(Error::ReadOnlyFS)
    }

    fn write(&mut self, _file: INodeNum, _offset: u64, _buf: &[u8]) -> Result<usize> {
        Err(Error::ReadOnlyFS)
    }

    fn link(&mut self, _source: INodeNum, _parent: INodeNum, _name: &Path) -> Result<()> {
        Err(Error::ReadOnlyFS)
    }

    fn symlink(&mut self, _link: &Path, _parent: INodeNum, _name: &Path) -> Result<INodeNum> {
        Err(Error::ReadOnlyFS)
    }

    fn truncate(&mut self, _file: INodeNum, _size: u64) -> Result<()> {
        Err(Error::ReadOnlyFS)
    }
}

#[allow(dead_code, unused_variables)]
#[cfg(test)]
mod test {
    use super::*;
    use crate::block::block_core::test::block_from_file;
    use crate::vfs::Error::Unsupported;
    use crate::vfs::OwnedDirEntry;
    use core::mem::size_of;
    use std::env;
    use std::fs::File;
    use std::io::Cursor;
    use std::io::Read;

    fn open_disk_image(path: &str) -> Result<VSFS> {
        // Print current directory
        println!("Current directory: {:?}", env::current_dir().unwrap());

        // Try to create block device
        let mut file = File::open(path).unwrap();
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).expect("Failed to read file");
        let metadata = file.metadata().unwrap();
        println!("File size: {} bytes", metadata.len());

        let block = block_from_file(Cursor::new(buffer));
        let mut vsfs = VSFS::new(block).unwrap();
        println!("Successfully created VSFS");
        // print superblock's every field
        println!("Magic number: {:#x}", vsfs.superblock.magic_number);
        println!("File system size in bytes: {}", vsfs.superblock.fs_size);
        println!(
            "Total number of inodes (set by mkfs): {}",
            vsfs.superblock.num_inodes
        );
        println!(
            "Number of available inodes: {}",
            vsfs.superblock.free_inodes
        );
        println!("File system size in blocks: {}", vsfs.superblock.num_blocks);
        println!(
            "Number of available blocks: {}",
            vsfs.superblock.free_blocks
        );
        println!(
            "First block after inode table: {}",
            vsfs.superblock.data_start
        );

        // print out the inode bitmap
        //println!("Inode bitmap: {:?}", vsfs.inode_bitmap.bits);
        println!("Inode bitmap length: {}", vsfs.inode_bitmap.bits.len());
        // print out the inode bitmap's content, only when the bit is 1
        for i in 0..64 {
            //println!("Inode bitmap {}: {:?}", i, vsfs.inode_bitmap.bits[i]);
        }

        // print out the data bitmap
        //println!("Data bitmap: {:?}", vsfs.data_bitmap.bits);
        println!("Data bitmap length: {}", vsfs.data_bitmap.bits.len());

        // print out inode table
        for i in 0..vsfs.inodes.len() {
            // print out only allocated inodes in the inode bitmap
            //println!("Inode {}: {:?}", i, vsfs.inodes[i]);
            if vsfs.inode_bitmap.is_allocated(i as u32) {
                println!("Inode {}: {:?}", i, vsfs.inodes[i]);
            }

            //println!("Inode {}: {:?}", i, vsfs.inodes[i]);
        }

        // print VSFS_BLOCK_SIZE divided by inode size
        println!("VSFS_BLOCK_SIZE / inode size: {}", size_of::<Inode>());

        println!("{}", vsfs.inodes.len());

        // Read the first inode's data and print it out
        let root = SimpleFileSystem::root(&vsfs);
        vsfs.open(root).unwrap();

        // print root inode's data
        let root_inode = vsfs.inodes[root as usize];
        println!("Root inode: {:?}", root_inode);

        let entries: Vec<OwnedDirEntry> = SimpleFileSystem::readdir(&mut vsfs, root)
            .unwrap()
            .to_sorted_vec();
        for entry in entries {
            println!("Entry: {:?}", entry);
        }
        let mut buf = [0; 4096 * 8];
        let n = vsfs.read(3, 4096 * 7 + 512, &mut buf[..]).unwrap();
        println!("Read size n: {}", n);
        //println!("second inode's data: {:?}", &buf);

        // print buf content as a char
        for i in 4096 * 7..4096 * 7 + 512 {
            print!("{}", buf[i] as char);
        }
        println!();

        // Return error always for now
        Err(Unsupported)
    }

    // #[test]
    fn test_1file() {
        let image_path = "tests/vsfs/vsfs-1file.disk";

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
    fn test_3file() {
        let image_path = "tests/vsfs/vsfs-3file.disk";

        // Check if file exists
        if !std::path::Path::new(image_path).exists() {
            panic!("Test image file not found at path: {}", image_path);
        }

        let vsfs = open_disk_image(image_path);
        // Rest of test...

        // Panic
        panic!("Test failed");
    }
    // //#[test]
    // fn test_3files() {
    //     let image_path = "src/fs/tests/vsfs/images/vsfs-3file.disk";

    //     // Check if file exists
    //     if !std::path::Path::new(image_path).exists() {
    //         panic!("Test image file not found at path: {}", image_path);
    //     }

    //     let vsfs = open_disk_image(image_path);
    //     // Rest of test...
    // }

    // //#[test]
    // fn test_empty() {
    //     let image_path = "src/fs/tests/vsfs/images/vsfs-empty.disk";

    //     // Check if file exists
    //     if !std::path::Path::new(image_path).exists() {
    //         panic!("Test image file not found at path: {}", image_path);
    //     }

    //     let vsfs = open_disk_image(image_path);
    //     // Rest of test...
    // }

    // //#[test]
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
