pub mod fat;
pub mod fs_manager;
pub mod syscalls;
pub mod vsfs;
use crate::fs::fs_manager::Mode;
use crate::system::{root_filesystem, running_process, running_thread_pid};
use crate::threading::process::Pid;
use crate::vfs::{Path, Result};
use alloc::{vec, vec::Vec};

pub type FileDescriptor = i16;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProcessFileDescriptor {
    pub pid: Pid,
    pub fd: FileDescriptor,
}

/// Read entire contents of file to kernel memory.
pub fn read_file(path: &Path) -> Result<Vec<u8>> {
    let mut root = root_filesystem().lock();
    let fd = root.open(&running_process().lock(), path, Mode::ReadWrite)?;
    let fd = ProcessFileDescriptor {
        fd,
        pid: running_thread_pid(),
    };
    let mut data = vec![];
    loop {
        let bytes_read = data.len();
        data.resize(bytes_read + 4096, 0);
        let n = root.read(fd, &mut data[bytes_read..])?;
        data.truncate(bytes_read + n);
        if n == 0 {
            break;
        }
    }
    Ok(data)
}

#[cfg(test)]
mod test {
    use super::*;
    use alloc::vec;
    use std::fs::File; 
    use crate::block::block_core::{test::block_from_file, BLOCK_SECTOR_SIZE, Block};
    use std::io::{prelude::*, Cursor};
    use std::io::{Read, Seek, SeekFrom};

    //#[test]
    fn block_read() {
    

    // let fat_path = "tests/fat/large_dir_fat16.img.gz";
    // let fat_file = File::open(fat_path).unwrap();
    // let mut gz_decoder = flate2::read::GzDecoder::new(fat_file);
    // let mut buf = vec![];
    // gz_decoder.read_to_end(&mut buf).unwrap();

    // // print 
    // println!("{:?}", &buf[0..100].to_vec());

    // let fat_block = block_from_file(Cursor::new(buf));

    // let mut first_sector = [0; BLOCK_SECTOR_SIZE];
    // fat_block.read(0, &mut first_sector);

    // // print first sector (the whole vector)
    // println!("{:?}", first_sector);

    // let mut first_sector_2 = [1; BLOCK_SECTOR_SIZE];
    // fat_block.read(1, &mut first_sector_2);

    // // print first sector (the whole vector)
    // println!("{:?}", first_sector_2);

    
    let image_path = "tests/vsfs/vsfs-1file.disk";

    // Check if the file exists
    if !std::path::Path::new(image_path).exists() {
        panic!("Test image file not found at path: {}", image_path);
    }

    // Open the file
    let mut file = File::open(image_path).expect("Failed to open file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");

    // print the file size
    println!("File size (using buffer): {} bytes", buffer.len());

    let block = block_from_file(Cursor::new(buffer));

    let metadata = file.metadata().unwrap();
    println!("File size: {} bytes", metadata.len());

    let mut first_sector = [0; BLOCK_SECTOR_SIZE];
    block.read(0, &mut first_sector);

    // print first sector
    for i in 0..BLOCK_SECTOR_SIZE {
        print!("{:02x} ", first_sector[i]);
    }

    let mut first_sector_again = [0; BLOCK_SECTOR_SIZE];
    block.read(0, &mut first_sector_again);

    // print first sector again
    print!("\n");    
    for i in 0..BLOCK_SECTOR_SIZE {
        print!("{:02x} ", first_sector_again[i]);
    }

    let mut nineth_sector = [0; BLOCK_SECTOR_SIZE];
    block.read(8, &mut nineth_sector);

    // print nineth sector
    print!("\n");
    for i in 0..BLOCK_SECTOR_SIZE {
        print!("{:02x} ", nineth_sector[i]);
    }

    // Panic
    panic!("Test failed");
  }
}
