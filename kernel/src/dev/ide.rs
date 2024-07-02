//ATA driver. Referenced from PINTOS devices/ide.c


use kidneyos_shared::println;
use alloc::{format, string::String};
const NUM_CHANNELS: usize = 2;

// Alternate Status Register Bits
const STA_BSY: u8 = 0x80;
const STA_DRDY: u8 = 0x40;
const STA_DRQ: u8 = 0x08;
// Control Register bits
const CTL_SRST:u8 = 0x04;
// Device Register bits
const DEV_MBS: u8 = 0xa0;
const DEV_LBA: u8 = 0x40;
const DEV_DEV: u8 = 0x10;
// Commands
const CMD_IDENTIFY_DEVICE: u8 = 0xec;
const CMD_READ_SECTOR_RETRY: u8 = 0x20;
const CMD_WRITE_SECTOR_RETRY: u8 = 0x30;

const BLOCK_SECTOR_SIZE: usize=512;

fn byte_enumerator(s:String) -> impl Iterator<Item=(usize,u8)>{
    s.into_bytes().into_iter().enumerate()
}

struct ATAChannel{
    name: [u8; 8], 
    reg_base: u16,
    irq: u8,
    channel_num: u8,
    //waiting on locks

    //disk data
    d0_name: [u8; 8], 
    d0_is_ata: bool,
    d1_name: [u8; 8],
    d1_is_ata: bool,
}


impl ATAChannel{
    /* ATA command block port addresses */
    fn reg_data(&self)-> u16{ self.reg_base + 0 }
    fn reg_error(&self)->u16{ self.reg_base + 1 }
    fn reg_nsect(&self)-> u16{ self.reg_base + 2 }
    fn reg_lbal(&self)->u16{ self.reg_base + 3 } 
    fn reg_lbam(&self)-> u16{ self.reg_base + 4 }
    fn reg_lbah(&self)->u16{ self.reg_base + 5 }
    fn reg_device(&self)-> u16{ self.reg_base + 6 }
    fn reg_status(&self)->u16{ self.reg_base + 9 }
    fn reg_command(&self)->u16{ self.reg_base }
    /* ATA control block port adresses */
    fn reg_ctl(&self)->u16{self.reg_base + 0x206}
    fn reg_alt_status(&self)->u16{self.reg_base + 0x206}

    fn new(channel_num :u8)->ATAChannel{
        
        let mut name: [u8;8] = [0; 8];
        for (j,c) in  byte_enumerator(format!("ide{}zu",channel_num)){
            name[j] = c;
        }
        let reg_base = match channel_num{
            0 => 0x1f0,
            1 => 0x170,
            _ => panic!(),
        };

        let irq = match channel_num{
            0 => 14 + 0x20,
            1 => 15 + 0x20,
            _ => panic!(),
        };       

        //initialize disks
        let mut d0_name: [u8;8] = [0; 8];
        let mut d1_name: [u8;8] = [0; 8];
        for (j,c) in  byte_enumerator(format!("hd{}",(61 + 0 + channel_num*2) as char)){
            d0_name[j] = c;
        }
        for (j,c) in  byte_enumerator(format!("hd{}",(61 + 1 + channel_num*2) as char)){
            d1_name[j] = c;
        }
        
        ATAChannel{
            name : name,
            reg_base: reg_base,
            irq: irq,
            channel_num: channel_num,
            d0_name: d0_name,
            d0_is_ata: false,
            d1_name: d1_name,
            d1_is_ata: false,
        }
    }

    fn check_device_type(&mut self, dev_num: u8) -> bool{
        
    }


    fn identify_ata_device(&mut self, dev_num: u8){
        
        let mut id[char; BLOCK_SECTOR_SIZE]


        if dev_num == 0{



        }else{

        }


    }
}

//call with interupts enabled
pub fn ide_init(){
    println!("Initialziing ATA driver in PIO mode");
    let mut channels: [ATAChannel; NUM_CHANNELS] = [ATAChannel::new(0), ATAChannel::new(1)];
}



use core::arch::asm;
fn outb(value :u8, port: u16){
    unsafe {
    asm!(
        "outb %al, %dx",
        in("dx") port,
        in("al") value,
        options(att_syntax)
     );
    };
}   


fn inb(port: u16)->u8{
    let mut ret: u8 = 0;
    unsafe {
        asm!(
            "inb %al, %dx",
            in("dx") port,
            out("al") ret,
            options(att_syntax)
        );
    }
    ret
}