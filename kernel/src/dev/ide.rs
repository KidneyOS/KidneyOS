//ATA driver. Referenced from PINTOS devices/ide.c


use kidneyos_shared::println;
use alloc::{format, string::String};
const NUM_CHANNELS: usize = 2;

struct ATADisk<'a>{
    name: [char; 8],
    channel: &'a ATAChannel<'a>,
    dev_no: u16,
    is_ata: bool,
}


struct ATAChannel<'a>{
    name: [char; 8], 
    reg_base: u16,
    irq: u8,

    //waiting on locks

    devices: [Option<ATADisk<'a>>; 2],
}

impl Default for ATAChannel<'_>{
    fn default()->ATAChannel<'static>{
        ATAChannel{
            name : ['\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0'],
            reg_base: 0,
            irq: 0,
            devices: [Option::None, Option::None],
        }
    }
}



pub fn ide_init(){
    println!("Initialziing ATA driver in PIO mode");

    let mut channels: [ATAChannel; NUM_CHANNELS] = [ATAChannel::default(), ATAChannel::default()];

    for i in 0..NUM_CHANNELS{


        let s: Cstr = format!("ide{}zu",i);
        for (j,c) in s.chars().enumerate(){
            channels[i].name[j] = c;
        }
        println!("{:?}",channels[i].name);
        
        channels[i].reg_base = match channel_no{
            0 => 0x1f0
            1 => 0x170
            _ => panic!()
        }


    }



}

unsafe fn outb(value :u8, port: u16){
    asm!(
        "outb %al, %dx",
        in("dx") port,
        in("al") value,
        options(att_syntax)
     );
}   
