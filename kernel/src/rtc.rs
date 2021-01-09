#![no_std]

use core::fmt;
use x86::io::{inb, outb};

pub const NMI_DISABLE: u8 = 1 << 7;
pub const SECOND_REG: u8 = 0x0;
pub const MINUTE_REG: u8 = 0x2;
pub const HOUR_REG: u8 = 0x4;
pub const WEEKDAY_REG: u8 = 0x6;
pub const DAY_REG: u8 = 0x7;
pub const MONTH_REG: u8 = 0x8;
pub const YEAR_REG: u8 = 0x9;
pub const CENTURY_REG: u8 = 0x32;
pub const STATUS_B_REG: u8 = 0xB;

pub const STATUS_B_ENCODING_MASK: u8 = 0x4;
pub const STATUS_B_HOUR_FORMAT_MASK: u8 = 0x2;

pub const CMOS_WRITE_CMD: u8 = 0x70;
pub const CMOS_READ_CMD: u8 = 0x71;

fn read_cmos_reg(reg: u8) -> u8 {
    unsafe {
        outb(0x70, NMI_DISABLE | reg);
        inb(0x71)
    }
}

fn from_bcd(bcd: u8) -> u8 {
    ((bcd / 16) * 10) + (bcd & 0xf)
}

struct CMOSDate {
    sec: u8,
    min: u8,
    hour: u8,
    day: u8,
    month: u8,
    year: u8,
    weekday: u8,
    century: u8,
    is_24_hour: bool,
    is_pm: bool,
}

impl fmt::Debug for CMOSDate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_24_hour {
            write!(
                f,
                "UTC Time: {:02}:{:02}:{:02} Date: {:02}/{:02}/{:04}",
                self.hour,
                self.min,
                self.sec,
                self.month,
                self.day,
                (self.year as u16) + (self.century as u16 * 100)
            )
        } else {
            write!(
                f,
                "UTC Time: {:02}:{:02}:{:02} {} Date: {:02}/{:02}/{:04}",
                self.hour,
                self.min,
                self.sec,
                if self.is_pm { "PM" } else { "AM" },
                self.month,
                self.day,
                (self.year as u16) + (self.century as u16 * 100)
            )
        }
    }
}

impl CMOSDate {
    pub fn new() -> CMOSDate {
        let status_b = read_cmos_reg(0xb);
        let is_bcd = { (status_b & STATUS_B_ENCODING_MASK) == 0 };
        let is_24_hour = { (status_b & STATUS_B_HOUR_FORMAT_MASK) == 1 };
        let mut is_pm = false;
        let sec;
        let min;
        let hour;
        let weekday;
        let day;
        let month;
        let year;
        let century;

        if is_bcd {
            sec = from_bcd(read_cmos_reg(SECOND_REG));
            min = from_bcd(read_cmos_reg(MINUTE_REG));
            if is_24_hour {
                hour = from_bcd(read_cmos_reg(HOUR_REG));
            } else {
                let hour_reg = read_cmos_reg(HOUR_REG);
                is_pm = ((hour_reg & 0x80) >> 7) == 1;
                hour = from_bcd(hour_reg & 0x7F);
            }
            weekday = from_bcd(read_cmos_reg(WEEKDAY_REG));
            day = from_bcd(read_cmos_reg(DAY_REG));
            month = from_bcd(read_cmos_reg(MONTH_REG));
            year = from_bcd(read_cmos_reg(YEAR_REG));
            century = from_bcd(read_cmos_reg(CENTURY_REG));
        } else {
            sec = read_cmos_reg(SECOND_REG);
            min = read_cmos_reg(MINUTE_REG);
            hour = read_cmos_reg(HOUR_REG);
            weekday = read_cmos_reg(WEEKDAY_REG);
            day = read_cmos_reg(DAY_REG);
            month = read_cmos_reg(MONTH_REG);
            year = read_cmos_reg(YEAR_REG);
            century = read_cmos_reg(CENTURY_REG);
        }
        CMOSDate {
            sec,
            min,
            hour,
            day,
            month,
            year,
            weekday,
            century,
            is_24_hour,
            is_pm,
        }
    }
}

pub fn print_date() {
    println!("{:?}", CMOSDate::new());
}
