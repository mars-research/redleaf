// Borrowed from https://github.com/gz/rust-perfcnt/blob/de296a732d8bdd8a94e7218422f6092dc77a01c9/src/bin/list.rs
//
use alloc::vec::Vec;
use x86::perfcnt::intel::{events, EventDescription};
use x86::cpuid::CpuId;

use core::str;

use core::fmt::{Error, Result, Write};

fn print_counter(id: &str, info: &EventDescription) {
    println!("{}:", id);

    let desc: &str = info.brief_description;
    let desc_words: Vec<&str> = desc.split(' ').collect();
    let mut chars = 0;
    print!("\t");
    for word in desc_words {
        if word.len() + chars > 60 {
            println!("");
            print!("\t");
            chars = 0;
        }
        print!("{} ", word);
        chars += word.len();
    }
    println!(" ");
    println!(" ");
}

const MODEL_LEN: usize = 30;

#[derive(Default)]
struct ModelWriter {
    buffer: [u8; MODEL_LEN],
    index: usize,
}

impl ModelWriter {
    fn as_str(&self) -> &str {
        str::from_utf8(&self.buffer[..self.index]).unwrap()
    }
}

impl Write for ModelWriter {
    fn write_str(&mut self, s: &str) -> Result {
        // TODO: There exists probably a more efficient way of doing this:
        for c in s.chars() {
            if self.index >= self.buffer.len() {
                return Err(Error);
            }
            self.buffer[self.index] = c as u8;
            self.index += 1;
        }
        Ok(())
    }
}


pub fn list_perf_cnt() {
    println!("All supported events on this hardware:");
    println!("----------------------------------------------------------");


    let cpuid = CpuId::new();

    cpuid.get_vendor_info().map_or(None, |vf| {
        cpuid.get_feature_info().map_or(None, |fi| {
            let vendor = vf.as_str();
            let (family, extended_model, model) =
                (fi.family_id(), fi.extended_model_id(), fi.model_id());

            let mut writer: ModelWriter = Default::default();
            // Should work as long as it fits in MODEL_LEN bytes:
            write!(writer, "{}-{}-{:X}{:X}", vendor, family, extended_model, model).unwrap();
            let key = writer.as_str();
            println!("{}", key);
            Some(())
        })
    });

    let cc = events();

    cc.map(|counters| {
        for (id, cd) in counters {
            print_counter(id, cd);
        }
    });

    let cc_count = cc.map(|c| c.len()).unwrap_or(0);
    println!("Total H/W performance events: {}", cc_count);
}
