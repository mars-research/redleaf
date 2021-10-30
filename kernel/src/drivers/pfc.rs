use core::str::FromStr;

use super::Driver;
use crate::redsys::IRQRegistrar;
use alloc::{string::String, sync::Arc,vec::Vec};
use spin::Mutex;
use rust_perfcnt_bare_metal::*;
use x86::{msr::*, perfcnt::intel::{ EventDescription,Counter,Tuple}};
use crate::interrupt::*;
use backtracer;

use rust_perfcnt_bare_metal::x86_intel::globle_ctrl::PERFCNT_GLOBAL_CTRLER;

pub struct PerfCount {
    event_name:String,
    rips:Vec<u64>,
    perf:PerfCounter,
    overflow_threshold: u64,
}

impl Driver for PerfCount {
    fn set_irq_registrar(&mut self, registrar: IRQRegistrar<PerfCount>) {
        // Request IRQ 185 (PerfCount overflow)
        registrar.request_irq(InterruptIndex::PfcOverflow as u8, PerfCount::pmc_overflow_handler).unwrap();
    }
}


impl PerfCount {
    pub fn new() -> PerfCount {
        unsafe{
        PerfCount {
            event_name : String::from("CPU_CLK_UNHALTED.THREAD"),
            rips : Vec::new(),
            perf : PerfCounter::new(&PERFCNT_GLOBAL_CTRLER),
            overflow_threshold:0,
        }
        }
    }

    ///Not used... For testing only.
    pub fn pmc_overflow_handler(&mut self) {
        disable_irq();
        println!("overflow interrupt!");
        unsafe{
        match PERFCNT_GLOBAL_CTRLER.get_overflow_counter().unwrap(){
            Counter::Fixed(index)=>{
                println!("Fixed PMC overflow index: {}", index);
            },
            Counter::Programmable(index)=>{
                println!("Programmable PMC overflow index: {}", index);
            },
        }
        PERFCNT_GLOBAL_CTRLER.reset_overflow_interrput();
        PERFCNT_GLOBAL_CTRLER.clear_overflow_bit(PERFCNT_GLOBAL_CTRLER.get_overflow_counter().unwrap());
        }
        enable_irq();
    }

    pub fn pmc_overflow_handler_direct(&mut self,pt_regs: &mut idt::PtRegs) {
        disable_irq();
        self.perf.stop();
        //self.rips.push(pt_regs.rip);

        let mut v:Vec<u64> = Vec::new();

        backtracer::trace_from(backtracer::EntryPoint::new(pt_regs.rbp,pt_regs.rsp,pt_regs.rip), |frame| {
            let ip = frame.ip();
            v.push(ip as u64);
            true        // xxx
        });

        loop{
            match v.pop(){
                Some(i) => {self.rips.push(i)},
                None => {break;},
            }
        }
        self.rips.push(0xFFFFFFFFFFFFFFFF);

        unsafe{
        match PERFCNT_GLOBAL_CTRLER.get_overflow_counter().unwrap(){
            Counter::Fixed(index)=>{
                //println!("Fixed PMC overflow index: {}", index);
            },
            Counter::Programmable(index)=>{
                //println!("Programmable PMC overflow index: {}", index);
            },
        }
        PERFCNT_GLOBAL_CTRLER.reset_overflow_interrput();
        PERFCNT_GLOBAL_CTRLER.clear_overflow_bit(PERFCNT_GLOBAL_CTRLER.get_overflow_counter().unwrap());
        }
        
        self.perf.overflow_after(self.overflow_threshold);
        self.perf.start();
        enable_irq();
    }
    
}


pub fn test_perfcount(){
    unsafe{
    PERFCNT_GLOBAL_CTRLER.init();
    PERFCNT_GLOBAL_CTRLER.register_overflow_interrput(InterruptIndex::PfcOverflow as u8+IRQ_OFFSET); //will go to pmc_overflow_handler() when testing
    let mut counter:PerfCounter = rust_perfcnt_bare_metal::x86_intel::PerfCounter::new(&PERFCNT_GLOBAL_CTRLER);
    
    
    println!("testing counters");
    let counter_description = x86::perfcnt::intel::events()
    .unwrap()
    .get("CPU_CLK_UNHALTED.THREAD")
    .unwrap();

    {
        let registrar = unsafe { get_irq_registrar(PERFCOUNTHDLER.clone()) };
        PERFCOUNTHDLER.lock().set_irq_registrar(registrar);
    }
    let index = 0;

    println!("version_id {}",PERFCNT_GLOBAL_CTRLER.get_version_identifier());
    println!("num_of_pmc {}",PERFCNT_GLOBAL_CTRLER.get_number_msr());
    println!("get_events_available {}",PERFCNT_GLOBAL_CTRLER.get_events_available());
    println!("get_bit_width {}",PERFCNT_GLOBAL_CTRLER.get_bit_width());
    println!("get_bit_width_fixed_counter {}",PERFCNT_GLOBAL_CTRLER.get_bit_width_fixed_counter());
    println!("get_number_fixed_function_counter {}",PERFCNT_GLOBAL_CTRLER.get_number_fixed_function_counter());
    println!("get_unavailable_events_vec {}",PERFCNT_GLOBAL_CTRLER.get_unavailable_events_vec());


    match counter.build_from_intel_hw_event(counter_description, index){
        Ok(_)=> {},
        Err(_)=> println!("Err when building"),
    };
    println!("before using : check_if_general_pmc_is_in_use {} ", PERFCNT_GLOBAL_CTRLER.check_if_general_pmc_is_in_use(index));

    println!("after building general mask is  {}",counter.get_general_pmc_mask());
    println!("after building fixed mask is  {}",counter.get_fixed_pmc_mask());

    counter.overflow_after(300);
    println!("overflow_status before overflow:{}",PERFCNT_GLOBAL_CTRLER.read_overflow_status());
    println!("reading before start overflow:{}",counter.read().unwrap());


    match counter_description.counter {
        Counter::Fixed(index) => {
            println!("counter index {}", index);
        },
        Counter::Programmable(_) => {},
    }
    println!("get_glbctlr_mask {:x}",PERFCNT_GLOBAL_CTRLER.read_globle_ctrl_bits().unwrap());

    counter.start();

    enable_irq();
    
    while PERFCNT_GLOBAL_CTRLER.read_overflow_status() == 0{
        //should generate an overflow_interrupt here.
        //println!("reading after start looping:{}",counter.read().unwrap());
    }
    disable_irq();
    }

}

pub fn startPerfCount(overflow_threshold:u64,counter_description:&EventDescription){
    println!("starting counters");    
    unsafe{
    PERFCNT_GLOBAL_CTRLER.init();
    PERFCNT_GLOBAL_CTRLER.register_overflow_interrput(InterruptIndex::PfcOverflow as u8);

    {
        let registrar = unsafe { get_irq_registrar(PERFCOUNTHDLER.clone()) };
        PERFCOUNTHDLER.lock().set_irq_registrar(registrar);
    }
    let index = 0;

    {
        PERFCOUNTHDLER.lock().overflow_threshold = overflow_threshold;
    }
    {PERFCOUNTHDLER.lock().perf.build_from_intel_hw_event(counter_description, 0);
    }
    
    let overflow_threshold = PERFCOUNTHDLER.lock().overflow_threshold;
    {
    PERFCOUNTHDLER.lock().perf.overflow_after(overflow_threshold);
    }
    {
    PERFCOUNTHDLER.lock().perf.start();
    }
    }
}

pub fn stopPerfCount(){
        PERFCOUNTHDLER.lock().perf.stop();
}

pub fn printPerfCountStats(){
    disable_irq();
    println!("Displaying Perf stats:");
    use crate::panic;
    let context = match panic::ELF_CONTEXT.r#try() {
        Some(t) => t,
        None => {
            println!("ELF_CONTEXT was not initialized");
            return;
        }
    };
    let relocated_offset = panic::RELOCATED_OFFSET;

    let mut s = String::from("");
    for rip in &PERFCOUNTHDLER.lock().rips{
        

        if *rip == 0xFFFFFFFFFFFFFFFF{
            s.push_str("\n");
            //println!("{}",s);
            //s = String::from("");
        }
        else{
        backtracer::resolve(context.as_ref(), relocated_offset,*rip as *mut u8, |symbol| {
            match symbol.name() {
                Some(fun_name) => {
                    //println!("rip: {} reslove {}", rip, s);
                    s.push_str(&fun_name);
                    s.push_str(";");
                    
                },
                None => {
                    //println!("rip: {} ", rip);
                },
            }
                   });
        }
    }
    println!("{}",s);
    println!("End Displaying Perf stats");
    enable_irq();
}



lazy_static! {
    pub static ref PERFCOUNTHDLER: Arc<Mutex<PerfCount>> = { Arc::new(Mutex::new(PerfCount::new())) };
}