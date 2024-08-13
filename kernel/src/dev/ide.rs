//ATA driver. Referenced from PINTOS devices/ide.c
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use super::super::sync::irq::MutexIrq;
use super::block::{BlockDevice, BlockSector, BlockType, BlockManager, BLOCK_SECTOR_SIZE};
use super::partition::{partition_scan};
use alloc::{format, str, string::String, boxed::Box};
use core::{arch::asm, ptr};
use kidneyos_shared::println;


const NUM_CHANNELS: usize = 2;
// Alternate Status Register Bits
const STA_BSY: u8 = 0x80;
const STA_DRDY: u8 = 0x40;
const STA_DRQ: u8 = 0x08;
// Control Register bits
const CTL_SRST: u8 = 0x04;
// Device Register bits
const DEV_MBS: u8 = 0xa0;
const DEV_LBA: u8 = 0x40;
const DEV_DEV: u8 = 0x10;
// Commands
const CMD_IDENTIFY_DEVICE: u8 = 0xec;
const CMD_READ_SECTOR_RETRY: u8 = 0x20;
const CMD_WRITE_SECTOR_RETRY: u8 = 0x30;

// Global IDE Devices
static CHANNEL_0: MutexIrq<ATAChannel> = MutexIrq::new(ATAChannel::new(0));
static CHANNEL_1: MutexIrq<ATAChannel> = MutexIrq::new(ATAChannel::new(1));

fn msleep(t: usize) {
    for _ in 0..9 {
        usleep(t);
    }
}

fn usleep(t: usize) {
    for _ in 0..9 {
        nsleep(t);
    }
}
fn nsleep(t: usize) {
    unsafe {
        asm!("nop");
    }
}

#[naked]
pub unsafe extern "C" fn ide_interrupt_handler() -> ! {
    fn inner() -> ! {
        panic!("PIO Interrupt");
    }

    asm!(
        "call {}",
        sym inner,
        options(noreturn),
    );
}

fn byte_enumerator(s: String) -> impl Iterator<Item = (usize, u8)> {
    s.into_bytes().into_iter().enumerate()
}

struct ATAChannel {
    name: [u8; 8],
    reg_base: u16,
    irq: u8,
    channel_num: u8,
    //waiting on locks
    expecting_interrupt: bool,
    //disk data
    d0_name: [u8; 8],
    d0_is_ata: bool,
    d1_name: [u8; 8],
    d1_is_ata: bool,
}


#[derive(Clone)]
pub struct ATADisk <'a> {
    channel: &'a MutexIrq<ATAChannel>,
    dev_no: u8,
    size: usize,
    name: String,
    idx: usize,
}

impl BlockDevice for ATADisk<'_> {
    fn block_read(&self, sec_no: BlockSector, buf: &mut [u8]) {
        let c: &mut ATAChannel = &mut self.channel.lock();
        c.select_sector(self.dev_no, sec_no);
        c.issue_pio_command(CMD_READ_SECTOR_RETRY);
        c.wait_until_ready();
        c.wait_while_busy();
        unsafe {
            c.read_sector(buf);
        }
    }

    fn block_write(&self, sec_no: BlockSector, buf: &[u8]) {
        let c: &mut ATAChannel = &mut self.channel.lock();
        c.select_sector(self.dev_no, sec_no);
        c.issue_pio_command(CMD_WRITE_SECTOR_RETRY);
        c.wait_until_ready();
        c.wait_while_busy();
        unsafe {
            c.write_sector(buf);
        };
    }
    fn block_type(&self) -> BlockType {
        BlockType::BlockRaw
    }
    fn block_size(&self) -> usize {
        self.size 
    }
    fn block_name(&self) -> &str{
        &self.name
    }

    fn block_set_idx(&mut self, idx: usize) {
        self.idx = idx; 
    }
    fn block_idx(&self) -> usize {
        self.idx
    }

}

impl ATAChannel {
    /* ATA command block port addresses */
    const fn new(channel_num: u8) -> ATAChannel {
        let name: [u8; 8] = [0; 8];
        let reg_base = match channel_num {
            0 => 0x1f0,
            1 => 0x170,
            _ => panic!(),
        };
        let irq = match channel_num {
            0 => 14 + 0x20,
            1 => 15 + 0x20,
            _ => panic!(),
        };
        //initialize disks
        let d0_name: [u8; 8] = [0; 8];
        let d1_name: [u8; 8] = [0; 8];
        ATAChannel {
            name,
            reg_base,
            irq,
            channel_num,
            expecting_interrupt: false,
            d0_name,
            d0_is_ata: false,
            d1_name,
            d1_is_ata: false,
        }
    }

    fn set_names(&mut self) {
        for (j, c) in byte_enumerator(format!("ide{}zu\0", self.channel_num)) {
            self.name[j] = c;
        }
        for (j, c) in byte_enumerator(format!("hd{}\0", (0x61 + self.channel_num * 2) as char)) {
            self.d0_name[j] = c;
        }
        for (j, c) in byte_enumerator(format!("hd{}\0", (0x61 + 1 + self.channel_num * 2) as char)) {
            self.d1_name[j] = c;
        }
    }
    
    fn set_is_ata(&mut self, dev_no: u8, is_ata: bool) {
        if dev_no == 0 {
            self.d0_is_ata = is_ata;
        } else {
            self.d1_is_ata = is_ata;
        }
    }

    fn is_ata(&self, dev_no: u8) -> bool {
        if dev_no == 0 {
            self.d0_is_ata
        } else {
            self.d1_is_ata
        }
    }
    
    // ATA Registers

    const fn reg_data(&self) -> u16 {
        self.reg_base
    }
    const fn reg_error(&self) -> u16 {
        self.reg_base + 1
    }
    const fn reg_nsect(&self) -> u16 {
        self.reg_base + 2
    }
    const fn reg_lbal(&self) -> u16 {
        self.reg_base + 3
    }
    const fn reg_lbam(&self) -> u16 {
        self.reg_base + 4
    }
    const fn reg_lbah(&self) -> u16 {
        self.reg_base + 5
    }
    const fn reg_device(&self) -> u16 {
        self.reg_base + 6
    }
    const fn reg_status(&self) -> u16 {
        self.reg_base + 7
    }
    const fn reg_command(&self) -> u16 {
        self.reg_base + 7
    }
    /* ATA control block port adresses */
    const fn reg_ctl(&self) -> u16 {
        self.reg_base + 0x206
    }
    const fn reg_alt_status(&self) -> u16 {
        self.reg_base + 0x206
    }
    fn issue_pio_command(&mut self, command: u8) {
        self.expecting_interrupt = true;
        outb(self.reg_command(), command);
    }


    // Basic ATA Functionality    
    fn wait_while_busy(&self) -> bool {
        for i in 0..3000 {
            if (inb(self.reg_alt_status()) & STA_BSY) == 0 {
                // println!("ok");
                return (inb(self.reg_alt_status()) & STA_DRQ) != 0;
            }
            usleep(10);
        }
        println!("failed with status {}", inb(self.reg_alt_status()));
        false
    }

    // polls device until idle
    fn wait_until_ready(&self) {
        for i in 0..3000 {
            // println!("waiting");
            let b = inb(self.reg_status());
            if (b & (STA_BSY | STA_DRQ)) == 0 {
                return;
            }
            msleep(10);
        }
    }


    fn select_device(&self, dev_num: u8) {
        let mut dev: u8 = DEV_MBS;
        if dev_num == 1 {
            dev |= DEV_DEV;
        }
        outb(self.reg_device(), dev);
        let status = inb(self.reg_alt_status());
        nsleep(400);
    }

    //Waits until channel is ready after selecting device
    fn select_device_wait(&self, dev_num: u8) {
        self.wait_until_ready();
        self.select_device(dev_num);
        self.wait_until_ready();
    }

    fn select_sector(&self, dev_no: u8, sector: BlockSector) {
        self.select_device_wait(dev_no);
        outb(self.reg_nsect(), 1);
        outb(self.reg_lbal(), sector as u8);
        outb(self.reg_lbam(), (sector >> 8) as u8);
        outb(self.reg_lbah(), (sector >> 16) as u8);
        outb(
            self.reg_device(),
            DEV_MBS
                | DEV_LBA
                | {
                    if dev_no == 1 {
                        dev_no
                    } else {
                        0
                    }
                }
                | ((sector >> 24) as u8),
        );



    }

    unsafe fn read_sector(&self, buf: &mut [u8]) {
        let ptr: *mut u8 = buf.as_mut_ptr();
        insw(self.reg_data(), ptr, BLOCK_SECTOR_SIZE / 2);
    }

    unsafe fn write_sector(&mut self, buf: &[u8]) {
        let ptr: *const u8 = buf.as_ptr();
        outsw(self.reg_data(), ptr, BLOCK_SECTOR_SIZE / 2);
    }

    fn reset_channel(&mut self) {
        let mut present: [bool; 2] = [false; 2];

        for dev_num in 0..2 {
            self.select_device(dev_num);
            outb(self.reg_nsect(), 0x55);
            outb(self.reg_lbal(), 0xaa);

            outb(self.reg_nsect(), 0xaa);
            outb(self.reg_lbal(), 0x55);

            outb(self.reg_nsect(), 0x55);
            outb(self.reg_lbal(), 0xaa);
            present[dev_num as usize] =
                (inb(self.reg_nsect()) == 0x55) && inb(self.reg_lbal()) == 0xaa;
        }
        outb(self.reg_ctl(), 0);
        usleep(10);
        outb(self.reg_ctl(), CTL_SRST);
        usleep(10);
        outb(self.reg_ctl(), 0);
        msleep(150);

        if present[0] {
            self.select_device(0);
            self.wait_while_busy();
        }
        if present[1] {
            self.select_device(1);
            for i in 0..3000 {
                if inb(self.reg_nsect()) == 1 && inb(self.reg_lbal()) == 1 {
                    break;
                }
                msleep(10);
            }
            self.wait_while_busy();
        }
    }

    fn check_device_type(&mut self, dev_num: u8) -> bool {
        self.select_device(dev_num);
        let error = inb(self.reg_error());
        let lbam = inb(self.reg_lbam());
        let lbah = inb(self.reg_lbah());
        let status = inb(self.reg_status());
        // println!("error: {} status: {}", error, status);
        if (error != 1 && (error != 0x81 || dev_num == 1))
            || (status & STA_DRDY) == 0
            || (status & STA_BSY) != 0
        {
            self.set_is_ata(dev_num, false);
            error != 0x81
        } else {
            self.set_is_ata(
                dev_num,
                (lbam == 0 && lbah == 0) || (lbam == 0x3c && lbah == 0xc3),
            );
            true
        }
    }

    
}


/* 
Identify ATA device using "identify" command, register block device and partitions
*/
fn identify_ata_device<'a>(channel: &'static MutexIrq<ATAChannel>, dev_no: u8, all_blocks: &'a mut BlockManager) {
        // block_sector_t capacity;
        let size: usize;
        let name: String;

        
        {
            let c: &mut ATAChannel = &mut channel.lock();
            let mut id: [u8; BLOCK_SECTOR_SIZE] = [0; BLOCK_SECTOR_SIZE];
            c.select_device_wait(dev_no);
            c.issue_pio_command(CMD_IDENTIFY_DEVICE);

            // self.wait_until_ready();
            if !c.wait_while_busy() {
                c.set_is_ata(dev_no, false);
                println!("channel {} device {} is not ata", c.channel_num, dev_no);
                return;
            }
            unsafe {
                c.read_sector(&mut id);
            }

            size = usize::from_le_bytes(id[120..124].try_into().unwrap());
            name = String::from_utf8({
                if dev_no == 0 {
                    c.d0_name
                } else {
                    c.d1_name
                }

            }.to_vec()).unwrap();
            println!(
                "channel: {} device: {} name: {} capacity: {}",
                c.channel_num,
                dev_no,
                &name,
                size.wrapping_mul(512),
            );
        }
        let d: ATADisk = ATADisk{
            channel: channel,
            dev_no,
            size,
            name,
            idx: 0,
        };
        all_blocks.register_block(Box::new(d.clone()));
        partition_scan(&d.clone(), all_blocks);
    }

//call with interupts enabled
pub fn ide_init(all_blocks: &mut BlockManager) {
    println!("Initialziing ATA driver in PIO mode");
    let mut channels: [&MutexIrq<ATAChannel>; NUM_CHANNELS] = [&CHANNEL_0, &CHANNEL_1];
    let mut present: [[bool; 2];2] = [[false, false], [false, false]];
    for (i, chan) in channels.iter().enumerate() {
        let c = &mut chan.lock();

        c.set_names();
        c.reset_channel();
        if c.check_device_type(0) {
            c.check_device_type(1);
        }
        
        for j in 0..2 {
            present[i][j] = c.is_ata(j as u8);
        }
    }

    for (i,c) in channels.iter().enumerate(){
        
        for j in 0..2 {
            if present[i][j] {
                identify_ata_device(c, j as u8, all_blocks);
            } else {
                println!("IDE: Channel {} device {} not present", i,j);
            }
        }
    }



    let mut test_sector: [u8; 512] = [10; 512];
    test_sector[0] = 2;
    println!("rw test result: {}", test_sector[0] == 10);
}


fn outb(port: u16, value: u8) {
    unsafe {
        asm!(
            "outb %al, %dx",
            in("al") value,
            in("dx") port,
            options(att_syntax),
        );
    };
}

fn inb(port: u16) -> u8 {
    let mut ret: u8;
    unsafe {
        asm!(
            "inb %dx, %al",
            out("al") ret,
            in("dx") port,
            options(att_syntax),
        );
    }
    ret
}

unsafe fn insw(port: u16, buf: *mut u8, count: usize) {
    asm!(
        "push edi",
        "mov edi, eax",
        "rep insw",
        "pop edi",
        in("dx") port,
        in("eax") buf,
        in("ecx") count,
    );
}

unsafe fn outsw(port: u16, buf: *const u8, count: usize) {
    asm!(
        "push esi",
        "mov esi, eax",
        "rep outsw",
        "pop esi",
        in("dx") port,
        in("eax") buf,
        in("ecx") count,
    );
}
