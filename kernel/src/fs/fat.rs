use alloc::{vec, string::String};
use core::ffi;
use crate::dev::block::{Block, BlockType};
use crate::fs::vfs::IOError;
use core::mem;
pub struct Fat16<'a> {
    block: &'a Block,
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    num_reserved_sectors: u16,
    num_fats: u8, // not sure if needed
    num_root_dir_entries: u16, // not sure if needed
    total_sectors: u32,
    num_sectors_per_fat: u16, 
    num_hidden_sectors: u32, // might be needed to locate FAT table
    //data_cluster_start: u16,
    data_sector_start: u16,
    fat_cache: Vec<u16>,
}
pub struct File {pub name: String,
    cluster: u16,
    pub file_size: u32,
    cache_offset: u32,
    cache_cluster: u16}
#[derive(Debug)]
pub struct Directory{
    pub name:String,
    cluster: u16,
    pub children: Vec<Inode>}
pub enum FSEntry{
    F(File),
    Dir(Directory),
}
#[derive(Debug)]
pub struct Inode {
    pub name: String,
    cluster: u16,
    file_size: u32,
    is_dir: bool
}




// NOTE: I haven't found directories without file size = 0

impl<'a> Fat16<'a> {
    pub fn new(block: &Block) -> Result<Fat16, IOError> {
        let mut buf = vec![0 as u8; 512];
        block.block_read(0, &mut buf);
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

        let mut fat_cache: Vec<u16> = vec![1 as u16; (num_sectors_per_fat as usize * bytes_per_sector as usize / 2 ) as usize];
        for i in 0..num_sectors_per_fat {
            block.block_read((num_reserved_sectors + i) as u32, &mut buf);
            for j in 0..(bytes_per_sector as usize/2) {fat_cache[(i*(bytes_per_sector/2)) as usize + j] = u16::from_le_bytes([buf[2*j], buf[2*j + 1]]); }

        }
        Ok( Fat16 {
            block, 
            bytes_per_sector,
            sectors_per_cluster,
            num_reserved_sectors,
            num_fats,
            num_root_dir_entries,
            total_sectors,
            num_sectors_per_fat,
            num_hidden_sectors,
            data_sector_start: (num_reserved_sectors + ((num_fats as u16) * num_sectors_per_fat))  + (num_root_dir_entries * 32 / bytes_per_sector), 
            fat_cache,
        } )
    }
    pub fn get_root(&self) -> Result<Directory, IOError> {
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
    pub fn get_inode(&self, location: &Inode) -> Result<FSEntry, IOError> {
        if location.is_dir {
            Ok(FSEntry::Dir(self.get_dir(location)?))
        } 
        else {
            Ok(FSEntry::F(self.get_file(location)?))
        }
    }
    // why -2 clusters, I don't know
    // for fat16, total sectors can't exceed 31 bits
    fn disk_read(&self, buf: &mut [u8], sector: isize){ 
        self.block.block_read((((self.data_sector_start - self.sectors_per_cluster as u16 * 2)) as isize + sector) as u32, buf) 
    }
    pub fn read_file_at(&self, file: &mut FSEntry, offset: u32, out: &mut [u8], mut amount: u32) -> Result<u32, IOError>{
        match file {
            FSEntry::Dir(_) => { Err( IOError::new("trying to read a directory".to_string())) } //TODO: maybe return a formatted list of children later
            FSEntry::F(file) => { 
                let cluster = file.cluster;
                let file_size: u32 = file.file_size;
                if offset + amount > file_size { amount = file_size - offset; } 
                let mut curr_cluster: u16;
                if offset == file.cache_offset { curr_cluster = file.cache_cluster; }
                else { curr_cluster = self.get_curr_cluster(cluster, offset); }
                let mut curr_sector: u32 = curr_cluster as u32 * self.sectors_per_cluster as u32 + ((offset / self.bytes_per_sector as u32) % self.sectors_per_cluster as u32 );
                let mut amount_read: u32 = 0;
                
                let mut buf = vec![0 as u8; self.bytes_per_sector as usize];
                if (offset + amount) / (self.bytes_per_sector + 1) as u32 > 0 { 
                    //first, align to sector
                    self.disk_read(&mut buf, curr_sector as isize); 
                    for i in offset % self.bytes_per_sector as u32 .. self.bytes_per_sector as u32 { out[(i - offset) as usize]= buf[i as usize]; } 
                    amount_read = self.bytes_per_sector as u32 - (offset % self.bytes_per_sector as u32);
                    curr_sector += 1;
                    if curr_sector % self.sectors_per_cluster as u32 == 0 { curr_cluster = self.fat_cache[curr_cluster as usize]; curr_sector = curr_cluster as u32 * self.sectors_per_cluster as u32;}
                    
                    while (amount_read / self.bytes_per_sector as u32) < (amount / self.bytes_per_sector as u32) {
                        self.disk_read(&mut buf, curr_sector as isize);
                        for i in 0..self.bytes_per_sector as u32 { out[(amount_read + i) as usize] = buf[i as usize];}
                        amount_read += self.bytes_per_sector as u32;
                        curr_sector += 1;
                        if curr_sector % self.sectors_per_cluster as u32 == 0 { curr_cluster = self.fat_cache[curr_cluster as usize]; curr_sector = curr_cluster as u32 * self.sectors_per_cluster as u32; }
                    }
                }
                self.disk_read(&mut buf, curr_sector as isize);
                for i in 0..(amount - amount_read) { out[(amount_read + i) as usize] = buf[((offset + amount_read) % self.bytes_per_sector as u32) as usize + i as usize]; }
                
                if (amount + offset) % self.bytes_per_sector as u32 == 0 { curr_sector += 1; } 
                if curr_sector % self.sectors_per_cluster as u32 == 0 && curr_cluster != file.cluster { curr_cluster = self.fat_cache[curr_cluster as usize];}
                file.cache_offset = offset + amount;
                file.cache_cluster = curr_cluster;
                Ok(amount)
            }
        }
    }
    pub fn get_dir(&self, dir: &Inode) -> Result<Directory, IOError>{
        fn add_to_name_unsafe(name: &mut String, buf: &[u8]) {
            // the more efficient way
            assert_eq!(buf.len(), 32);
            // align seems to stick to even boundaries when we specifically want an odd boundary. 
            // transmute reads ahead, but it's not an issue here because it's still within buf
            let u16buf: &[u16] = unsafe { mem::transmute(&buf[1..11]) };
            name.push_str(&String::from_utf16_lossy(&u16buf[0..5]));
            let (_, u16buf, _) = unsafe { buf.align_to::<u16>() };
            name.push_str(&String::from_utf16_lossy(&u16buf[7..13]));
            name.push_str(&String::from_utf16_lossy(&u16buf[14..16]));
        }
        fn add_to_name(name: &mut String, buf: &[u8]){
            // couldn't think of a cleaner way without unsafe/nightly rust
            assert_eq!(buf.len(), 32);
            name.push_str(&String::from_utf16_lossy(&([
                u16::from_le_bytes([buf[1], buf[2]]), u16::from_le_bytes([buf[3], buf[4]]),
                u16::from_le_bytes([buf[5], buf[6]]), u16::from_le_bytes([buf[7], buf[8]]),
                u16::from_le_bytes([buf[9], buf[10]])
                ])));
            name.push_str(&String::from_utf16_lossy(&([
                u16::from_le_bytes([buf[14], buf[15]]), u16::from_le_bytes([buf[16], buf[17]]),
                u16::from_le_bytes([buf[18], buf[19]]), u16::from_le_bytes([buf[20], buf[21]]),
                u16::from_le_bytes([buf[22], buf[23]]), u16::from_le_bytes([buf[24], buf[25]])
                ])));
            name.push_str(&String::from_utf16_lossy(&([
                u16::from_le_bytes([buf[28], buf[29]]), u16::from_le_bytes([buf[30], buf[31]]),
                ])));
        }

        let mut buf = vec![0 as u8; self.bytes_per_sector as usize]; 
        let mut curr_sector: isize = match dir.name.as_str() { "root" => -(self.num_root_dir_entries as isize * 32 / self.bytes_per_sector as isize) + (self.sectors_per_cluster as isize * 2) ,
            _ => dir.cluster as isize * self.sectors_per_cluster as isize };
        self.disk_read(&mut buf, curr_sector);
        let mut children: Vec<Inode> = Vec::new();
        let mut i = 0;
        const LFN_BYTES: [usize; 13] = [1,3,5,7,9,14,16,18,20,22,24,28,30];
        let mut long_name: Option<String> = None;
        while buf[i] != 0x00 { // last entry in dir table
            if buf[i] == 0xE5 { // empty directory
            }
            else if buf[i + 11] == 0b1111 { // handling long file names
                let num_lfn_entries = (buf[i] - 0x40) as usize;
                let mut name = String::with_capacity(num_lfn_entries * 13);
                let old_i = i;
                i += (num_lfn_entries as usize - 1) * 32;

                let mut bufs: [Vec<u8>;3] = [vec![], vec![], vec![]];
                bufs[0] = buf;
                for j in 1..(i/self.bytes_per_sector as usize + 1) {
                    let tempbuf = vec![0 as u8; self.bytes_per_sector as usize];
                    buf = tempbuf;
                    curr_sector += 1;
                    if curr_sector > 0 && curr_sector as u32 % self.sectors_per_cluster as u32 == 0 { curr_sector = self.fat_cache[ (curr_sector as usize / self.sectors_per_cluster as usize - 1)  as usize] as isize * self.sectors_per_cluster as isize; }
                    self.disk_read(&mut buf, curr_sector);
                    bufs[j] = buf;
                }
                for _ in (2 * self.bytes_per_sector as usize / 32)..(i/32 + 1) {
                    add_to_name_unsafe(&mut name, &bufs[2][(i - 2 * self.bytes_per_sector as usize)..(i - 2 * self.bytes_per_sector as usize + 32)]);
                    i -= 32;
                } 
                for _ in (self.bytes_per_sector as usize / 32)..(i/32 + 1){
                    add_to_name_unsafe(&mut name, &bufs[1][(i - self.bytes_per_sector as usize)..(i - self.bytes_per_sector as usize + 32)]);
                    i -= 32;
                }
                for _ in (old_i/32)..(i/32 ) {
                    add_to_name_unsafe(&mut name, &bufs[0][i..(i+32)]);
                    i -= 32;
                }
                for j in 0..12 {
                    if bufs[0][i + LFN_BYTES[j]] == 0 && bufs[0][i + LFN_BYTES[j] + 1] == 0 && bufs[0][i + LFN_BYTES[j+1]] == 0xff && bufs[0][i + LFN_BYTES[j+1]] == 0xff { break; }
                    name.push_str(&String::from_utf16_lossy(&[u16::from_le_bytes([bufs[0][i + LFN_BYTES[j]], bufs[0][i + LFN_BYTES[j] + 1]])]));
                }
                if bufs[0][i + LFN_BYTES[12]] != 0 && bufs[0][i + LFN_BYTES[12]] != 0xff && bufs[0][i + LFN_BYTES[12] + 1] != 0 && bufs[0][i + LFN_BYTES[12] + 1] != 0xff {
                    name.push_str(&String::from_utf16_lossy(&[u16::from_le_bytes([bufs[0][i + LFN_BYTES[12]], bufs[0][i + LFN_BYTES[12] + 1]])]));
                }
                assert_eq!(i, old_i);
                
                i = i + (num_lfn_entries - 1) * 32; 
                buf = mem::take(&mut bufs[i/self.bytes_per_sector as usize]);
                i = i % self.bytes_per_sector as usize; 
                long_name = Some(name);
            } 
            else {
                children.push(self.build_inode(&buf[i..i+32])); 
                let last = children.len() - 1;
                if let Some(name) = long_name { children[last].name = name; long_name = None; }
            }

            i += 32;
            if i % self.bytes_per_sector as usize == 0 { 
                curr_sector += 1;
                if curr_sector > 0 && curr_sector as u32 % self.sectors_per_cluster as u32 == 0 { curr_sector = self.fat_cache[ (curr_sector as usize / self.sectors_per_cluster as usize - 1)  as usize] as isize * self.sectors_per_cluster as isize; }
                self.disk_read(&mut buf, curr_sector);
                i = 0;
            }
        }
        Ok( Directory{name: dir.name.clone(), cluster: dir.cluster as u16, children})
    }
    pub fn get_file(&self, inode: &Inode) -> Result<File, IOError> {
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
        let mut name: String = {
            let mut i = 7;
            while bytes[i] == 0x20 { i -= 1; }
            let slice = &bytes[0..(i + 1)];
            parse_cstr(slice).to_string()

        }; 
        if bytes[8] != 0x20 {
            name.push('.');
            name.push_str(&{
                let slice = if bytes[9] == 0x20  { &bytes[8..9] }
                else if bytes[10] == 0x20 { &bytes[8..10] }
                else { &bytes[8..11] };
                parse_cstr(slice).to_string()
            });
        }
        let cluster = u16::from_le_bytes([bytes[26], bytes[27]]);
        let file_size = u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]); 
        if bytes[11] & 0b00010000 == 0b00010000 { Inode { name, cluster, file_size, is_dir: true } }
        else {Inode { name, cluster, file_size, is_dir: false }}
    }
}


fn parse_cstr(buf: &[u8]) -> &str {
    let mut l = 0;
    for i in 0..buf.len() {
        if buf[i] == 0 {
            break;
        }
        l += 1;
    }
    str::from_utf8(&buf[0..l]).unwrap()
    
}
