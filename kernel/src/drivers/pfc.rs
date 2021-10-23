use super::Driver;
use crate::redsys::IRQRegistrar;
use alloc::sync::Arc;
use spin::Mutex;
use rust_perfcnt_bare_metal::x86_intel::globle_ctrl::PerfCounterControler;
use rust_perfcnt_bare_metal::*;
use x86::{msr::*, perfcnt::intel::{ EventDescription,Counter,Tuple}};
use crate::interrupt::*;

use rust_perfcnt_bare_metal::x86_intel::globle_ctrl::PERFCNT_GLOBAL_CTRLER;
pub struct PerfCount {
}

impl Driver for PerfCount {
    fn set_irq_registrar(&mut self, registrar: IRQRegistrar<PerfCount>) {
        // Request IRQ 185 (PerfCount overflow)
        registrar.request_irq(185, PerfCount::pmc_overflow_handler).unwrap();
    }
}


impl PerfCount {
    pub fn new() -> PerfCount {
        PerfCount {
        }
    }

    pub fn pmc_overflow_handler(&mut self) {
        disable_irq();
        println!("overflow interrupt");
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
        }
        enable_irq();
    }
}


pub fn test_perfcount(){
    use rust_perfcnt_bare_metal::x86_intel::globle_ctrl::PERFCNT_GLOBAL_CTRLER;
    use super::pfc::{PERFCOUNTHDLER};
    use crate::drivers::Driver;

    
    unsafe{
    PERFCNT_GLOBAL_CTRLER.init();
    PERFCNT_GLOBAL_CTRLER.register_overflow_interrput(185+32);
    let mut counter:PerfCounter = rust_perfcnt_bare_metal::x86_intel::PerfCounter::new(&PERFCNT_GLOBAL_CTRLER);
    
    
    println!("testing counters");
    let counter_description = x86::perfcnt::intel::events()
    .unwrap()
    .get("BR_INST_RETIRED.ALL_BRANCHES")
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

    println!("after building mask is  {}",counter.get_general_pmc_mask());

    counter.overflow_after(300);
    println!("overflow_status before overflow:{}",PERFCNT_GLOBAL_CTRLER.read_overflow_status());
    println!("reading before start overflow:{}",counter.read().unwrap());

    counter.start();

    enable_irq();
    
    while PERFCNT_GLOBAL_CTRLER.read_overflow_status() == 0{

    }
    disable_irq();
    }

}


lazy_static! {
    pub static ref PERFCOUNTHDLER: Arc<Mutex<PerfCount>> = { Arc::new(Mutex::new(PerfCount::new())) };
}