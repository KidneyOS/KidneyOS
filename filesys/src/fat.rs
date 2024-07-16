use std::ffi::CString;
use std::io;
use crate::disk_device::{DiskDevice, Disk};
const MAX_SECTOR_SIZE: u16 = 4096;
pub struct Fat16<'a> {
    disk: &'a Disk,
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    num_reserved_sectors: u16,
    num_fats: u8, // not sure if needed
    num_root_dir_entries: u16, // not sure if needed
    total_sectors: u32,
    num_sectors_per_fat: u16, 
    num_hidden_sectors: u32, // might be needed to locate FAT table
    data_cluster_start: u16,
    fat_cache: Vec<u16>,
}
pub struct File {pub name: String,
    cluster: u16,
    pub file_size: u32,
    cache_offset: u32,
    cache_cluster: u16}
pub struct Directory{
    pub name:String,
    cluster: u16,
    pub children: Vec<Inode>}
pub enum FSEntry{
    F(File),
    Dir(Directory),
}
pub struct Inode {
    name: String,
    cluster: u16,
    file_size: u32,
    is_dir: bool
}

// NOTE: I haven't found directories without file size = 0

impl<'a> Fat16<'a> {
    pub fn new(disk: &Disk) -> Result<Fat16, io::Error> {
        //let mut buf: [u8; MAX_SECTOR_SIZE as usize] = [0; MAX_SECTOR_SIZE as usize];
        let mut buf = vec![0 as u8; 512];
        disk.read_at(&mut buf, 0)?;
        let num_reserved_sectors = u16::from_le_bytes([buf[14], buf[15]]);
        let num_sectors_per_fat = u16::from_le_bytes([buf[22], buf[23]]);
        let total_sectors = match u16::from_le_bytes([buf[19], buf[20]]) {
            0 => u32::from_le_bytes([buf[32], buf[33], buf[34], buf[35]]),
            _ => u16::from_le_bytes([buf[19], buf[20]]) as u32
        };
        let bytes_per_sector = u16::from_le_bytes([buf[11], buf[12]]);
        let sectors_per_cluster = buf[13];
        let num_fats = buf[16];
        let num_root_dir_entries = u16::from_le_bytes([buf[17], buf[18]]);
        let num_hidden_sectors = u32::from_le_bytes([buf[28], buf[29], buf[30], buf[31]]);

        let mut fat_cache: Vec<u16> = vec![1 as u16; (total_sectors / sectors_per_cluster as u32) as usize];
        //let mut fat_counter = 0;
        for i in 0..num_sectors_per_fat {
            disk.read_at(&mut buf, (num_reserved_sectors + i) as usize)?;
            for j in 0..(buf.len()/2) {fat_cache[(i*(bytes_per_sector/2)) as usize + j] = u16::from_le_bytes([buf[2*j], buf[2*j + 1]]); }

        }
        Ok( Fat16 {
            disk, 
            bytes_per_sector,
            sectors_per_cluster,
            num_reserved_sectors,
            num_fats,
            num_root_dir_entries,
            total_sectors,
            num_sectors_per_fat,
            num_hidden_sectors,
            data_cluster_start: (num_reserved_sectors + ((num_fats as u16) * num_sectors_per_fat)) / sectors_per_cluster as u16 + (num_root_dir_entries * 32 / bytes_per_sector / sectors_per_cluster as u16), 
            fat_cache,
        } )
    }
    pub fn get_root(&self) -> Result<Directory, io::Error> {
        let file_size: u32 = (((self.num_root_dir_entries * 32) + (self.bytes_per_sector - 1)) / self.bytes_per_sector) as u32;
        let mut res  = self.get_dir(
            &Inode { name: String::from("root"), 
            cluster: 0, // don't trust it
            file_size,
            is_dir: true})?;
            // simpler to remove the volume label entry from get_root than mod get_dir
        res.children.remove(0);
        Ok(res)
    }
    pub fn get_inode(&self, location: &Inode) -> Result<FSEntry, io::Error> {
        if location.is_dir {
            Ok(FSEntry::Dir(self.get_dir(location)?))
        } 
        else {
            Ok(FSEntry::F(self.get_file(location)?))
        }
    }

    // why -2? I have no idea
    fn disk_read(&self, buf: &mut [u8], sector: usize) -> Result<usize, io::Error> { 
        self.disk.read_at(buf, ((self.data_cluster_start - 2) * self.sectors_per_cluster as u16) as usize + sector) 
    }
    pub fn read_file_at(&self, file: &mut FSEntry, offset: u32, out: &mut [u8], mut amount: u32) -> Result<u32, io::Error>{
        match file {
            FSEntry::Dir(_) => { Err( io::Error::new(io::ErrorKind::InvalidInput, "trying to read a directory")) } //TODO: maybe return a formatted list of children later
            FSEntry::F(file) => { 
                let cluster = file.cluster;
                let file_size: u32 = file.file_size;
                if offset + amount > file_size { amount = file_size - offset; } 
                let mut curr_cluster: u16;
                if offset == file.cache_offset { curr_cluster = file.cache_cluster; }
                else { curr_cluster = self.get_curr_cluster(cluster, offset); }
                let mut curr_sector: u32 = curr_cluster as u32 * self.sectors_per_cluster as u32 + ((offset / self.bytes_per_sector as u32) % self.sectors_per_cluster as u32 );
                let mut amount_read: u32 = 0;
                
                let mut buf: [u8; MAX_SECTOR_SIZE as usize] = [0; MAX_SECTOR_SIZE as usize];
                if (offset + amount) / (self.bytes_per_sector + 1) as u32 > 0 { 
                    //first, align to sector
                    self.disk_read(&mut buf, curr_sector as usize)?; 
                    for i in offset % self.bytes_per_sector as u32 .. self.bytes_per_sector as u32 { out[(i - offset) as usize]= buf[i as usize]; } 
                    amount_read = self.bytes_per_sector as u32 - (offset % self.bytes_per_sector as u32);
                    curr_sector += 1;
                    if curr_sector % self.sectors_per_cluster as u32 == 0 { curr_cluster = self.fat_cache[curr_cluster as usize]; curr_sector = curr_cluster as u32 * self.sectors_per_cluster as u32;}
                    
                    while (amount_read / self.bytes_per_sector as u32) < (amount / self.bytes_per_sector as u32) {
                        self.disk_read(&mut buf, curr_sector as usize)?;
                        for i in 0..self.bytes_per_sector as u32 { out[(amount_read + i) as usize] = buf[i as usize];}
                        amount_read += self.bytes_per_sector as u32;
                        curr_sector += 1;
                        if curr_sector % self.sectors_per_cluster as u32 == 0 { curr_cluster = self.fat_cache[curr_cluster as usize]; curr_sector = curr_cluster as u32 * self.sectors_per_cluster as u32; }
                    }
                }
                self.disk_read(&mut buf, curr_sector as usize)?;
                for i in 0..(amount - amount_read) { out[(amount_read + i) as usize] = buf[i as usize]; }
                
                if (amount + offset) % self.bytes_per_sector as u32 == 0 { curr_sector += 1; } 
                if curr_sector % self.sectors_per_cluster as u32 == 0 && curr_cluster != file.cluster { curr_cluster = self.fat_cache[curr_cluster as usize];}
                file.cache_offset = offset + amount;
                file.cache_cluster = curr_cluster;
                Ok(amount)
            }
        }
    }
    pub fn get_dir(&self, dir: &Inode) -> Result<Directory, io::Error>{
        let mut buf: [u8; MAX_SECTOR_SIZE as usize] = [0; MAX_SECTOR_SIZE as usize];
        if dir.name == "root" {self.disk.read_at(&mut buf, (self.data_cluster_start as usize * self.sectors_per_cluster as usize) - (self.num_root_dir_entries * 32 / self.bytes_per_sector) as usize)?; }
        else {self.disk_read(&mut buf, dir.cluster as usize * self.sectors_per_cluster as usize)?;} // TODO: currently only reads 1 sector, change to cluster chain
        let mut children: Vec<Inode> = Vec::new();
        let mut i = 0;
        while buf[i] != 0x00 { // last entry in dir table
            if buf[i] == 0xE5 { // empty directory
                i += 32;
                continue;
            }
            if buf[i + 11] == 0b1111 { i += 32; continue; } // TODO: add LFN support
            children.push(self.build_inode(&buf[i..i+32])); // 32 bytes 
            i += 32;
        }
        Ok( Directory{name: dir.name.clone(), cluster: dir.cluster as u16, children: children})
    }
    pub fn get_file(&self, inode: &Inode) -> Result<File, io::Error> {
        Ok( File { name: inode.name.clone(), cluster: inode.cluster, file_size: inode.file_size, cache_offset: 0, cache_cluster: inode.cluster })
    }

    fn get_curr_cluster(&self, initial_cluster: u16, mut offset: u32) -> u16{
        let mut fat_offset = initial_cluster;
        while offset > (self.bytes_per_sector as u32 * self.sectors_per_cluster as u32){
            offset =  offset - (self.bytes_per_sector as u32 * self.sectors_per_cluster as u32);
            fat_offset = self.fat_cache[fat_offset as usize]; 
        }
        fat_offset
    }
    fn build_inode(&self, bytes: &[u8]) -> Inode{
        assert_eq!(bytes.len(), 32);
        let name = CString::new(&bytes[0..11]).unwrap(); // put this into its own function
        let cluster = u16::from_le_bytes([bytes[26], bytes[27]]);
        let file_size = u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]); 
        if bytes[11] & 0b00010000 == 0b00010000 { Inode { name: CString::into_string(name).expect("failed to create string"), cluster, file_size, is_dir: true } }
        else {Inode { name: CString::into_string(name).expect("failed to create string"), cluster, file_size, is_dir: false }}
    }
}


