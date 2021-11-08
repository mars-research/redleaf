use core::str::FromStr;
use super::Driver;
use crate::redsys::IRQRegistrar;
use alloc::{string::{String, ToString}, sync::Arc, vec::Vec};
use hashbrown::HashMap;
use spin::Mutex;
use rust_perfcnt_bare_metal::*;
use x86::{msr::*, perfcnt::intel::{ EventDescription,Counter,Tuple}};
use crate::interrupt::*;
use backtracer;

use rust_perfcnt_bare_metal::x86_intel::globle_ctrl::PERFCNT_GLOBAL_CTRLER;

const ENDOFSTACK:u64 = 0xFFFFFFFFFFFFFFFF;

lazy_static! {
    pub static ref PERFCOUNTHDLER: Arc<Mutex<PerfCount>> = { Arc::new(Mutex::new(PerfCount::new())) };
}

///This struct should not be used directly.
///
///To do a performance sampling:
/// 1. Call start_perf_count() 
/// 2. Run program 
/// 3. Call print_perf_count_stats
/// 4. Exit Readleaf and run ./perf-flame_graph_gen.sh
/// Note: Since the allocator is not lock-less. If too many samples are collected, Perf might call alloc() which might casue deadlock.
/// A reasonable number of samples should be less than 500000.
pub struct PerfCount {
    event_name:String,
    rips:Vec<u64>,
    buffer:Vec<u64>,
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
            event_name : String::from(""),
            rips : Vec::with_capacity(5000000),
            buffer : Vec::with_capacity(1000),
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
    
    //This is the interrupt handler for PMU overflow
    pub fn pmc_overflow_handler_direct(&mut self,pt_regs: &mut idt::PtRegs) {    
        disable_irq();
        //self.rips.push(pt_regs.rip);
        self.perf.stop();

        self.buffer.clear();
        
        backtracer::trace_from(backtracer::EntryPoint::new(pt_regs.rbp,pt_regs.rsp,pt_regs.rip), |frame| {
            let ip = frame.ip();
            self.buffer.push(ip as u64);
            true //go all the way to the bottom of the stack.        
        });

        if self.rips.capacity() - self.rips.len() < self.buffer.len() {
            println!("Perf's buffer has been filled. No more samples will be taken. Counters are stoped");
            //self.perf.overflow_after(self.overflow_threshold);
            //self.perf.start();
            enable_irq();
            return
        }

        for _ in 0..self.buffer.len(){
            self.rips.push(self.buffer.pop().unwrap());
        }

        self.rips.push(ENDOFSTACK);

        unsafe{
            /*match PERFCNT_GLOBAL_CTRLER.get_overflow_counter().unwrap(){
                Counter::Fixed(index)=>{
                    //println!("Fixed PMC overflow index: {}", index);
                },
                Counter::Programmable(index)=>{
                    //println!("Programmable PMC overflow index: {}", index);
                },
            }*/
        PERFCNT_GLOBAL_CTRLER.reset_overflow_interrput();
        PERFCNT_GLOBAL_CTRLER.clear_overflow_bit(PERFCNT_GLOBAL_CTRLER.get_overflow_counter().unwrap());
        }
        
        self.perf.overflow_after(self.overflow_threshold);
        self.perf.start();
        enable_irq();
    }
    
}

///Should generate an overflow_interrupt
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
    println!("IA32_PERF_CAPABILITIES : {}",PERFCNT_GLOBAL_CTRLER.get_perf_capability());


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

pub fn start_perf_count(overflow_threshold:u64,counter_description:&EventDescription){
    println!("starting counters");    
    unsafe{
    PERFCNT_GLOBAL_CTRLER.init();
    PERFCNT_GLOBAL_CTRLER.register_overflow_interrput(InterruptIndex::PfcOverflow as u8);

    {let registrar = unsafe { get_irq_registrar(PERFCOUNTHDLER.clone()) };
        PERFCOUNTHDLER.lock().set_irq_registrar(registrar);}

    let index = 0;
    {PERFCOUNTHDLER.lock().overflow_threshold = overflow_threshold;}
    {PERFCOUNTHDLER.lock().perf.build_from_intel_hw_event(counter_description, index);}
    let overflow_threshold = PERFCOUNTHDLER.lock().overflow_threshold;
    {PERFCOUNTHDLER.lock().perf.overflow_after(overflow_threshold);}
    {PERFCOUNTHDLER.lock().perf.start();}
    }
}

pub fn PausePerfCount(){
        PERFCOUNTHDLER.lock().perf.stop();
}

pub fn ResumePerfCount(){
    PERFCOUNTHDLER.lock().perf.start();
}

///This section is mainly for the reslover. 
/// After finishing collecting data using perf, use print_perf_count_stats to print parsed data to the serial.log 
/// After printing, end QEMU-KVM and run ./perf-flame_graph_gen.sh
/// As tested, 10000 lines of folded data needs around 2mins to print. (Disable WRITER and SERIAL1 in console.rs to improve perfermance)
/// TODO: Make the println!() faster.
/// 
struct RipToFunc{
    pub rip_left:u64,
    pub rip_right:u64,
    pub func_name:String,
}

pub fn print_perf_count_stats(){
    disable_irq();

    let mut func_names:Vec<RipToFunc> = Vec::new();

    println!("Processing Perf stats:");
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
    let mut map:HashMap<u64,String> = hashbrown::HashMap::new();
    let mut outmap:HashMap<String,u64> = hashbrown::HashMap::new();
    
    for rip in &PERFCOUNTHDLER.lock().rips{
        if *rip == ENDOFSTACK{
            //s.push_str("\n");
            //println!("{}",s);
            let mut count = outmap.entry(s).or_insert(0);
            *count += 1;
            s = String::from("");
        }
        else{
            let mut bingo= false;

            match map.get(rip){
                Some(fun_name)=>{
                    bingo = true;
                    s.push_str(fun_name);
                    s.push_str(";");
                },
                None=>{},
            }

            if ! bingo{
                match func_names.binary_search_by_key(rip,|f| f.rip_left){
                    Ok(i) => {
                        bingo = true;
                        s.push_str(&func_names[i].func_name);
                        s.push_str(";");
                        map.entry(*rip).or_insert(String::from_str(&func_names[i].func_name).unwrap());
                    },
                    Err(i) => {
                        if i >=1{
                            let left = &func_names[i-1];
                            if left.rip_left <= *rip && left.rip_right >= *rip{
                            //bingo!
                                bingo = true;
                                s.push_str(&left.func_name);
                                s.push_str(";");
                               map.entry(*rip).or_insert(String::from_str(&left.func_name).unwrap());
                            }
                        }
                    },
                }
            }

            if ! bingo{
                backtracer::resolve(context.as_ref(), relocated_offset,*rip as *mut u8, |symbol| {
                    match symbol.name() {
                        Some(fun_name) => {
                        //println!("rip: {} reslove {}", rip, s);
                            s.push_str(&fun_name);
                            s.push_str(";");

                            let mut bingo = false;
                            for i in 0..func_names.len(){
                                if func_names[i].func_name == fun_name{
                                    bingo = true;
                                    if rip < &func_names[i].rip_left{
                                        func_names[i].rip_left = *rip;
                                    }
                                    if rip > &func_names[i].rip_right{
                                       func_names[i].rip_right = *rip;
                                    }
                                    break;
                                }
                            }

                            if ! bingo{
                                func_names.push(RipToFunc{rip_left:*rip,rip_right:*rip,func_name:String::from_str(&fun_name).unwrap()});
                                func_names.sort_unstable_by_key(|s| s.rip_left);
                                map.entry(*rip).or_insert(String::from_str(&fun_name).unwrap());
                            }
                        },
                        None => {
                            //println!("rip: {} ", rip);
                        },
                    }
                });
            }
        }
    }
    let mut out = String::with_capacity(10000000);

    println!("Get Ready for {} lines of folded data.",outmap.len());
    for (k,v) in outmap{
        out.push_str(&k);
        out.push_str(" ");
        out.push_str(&v.to_string());
        out.push_str("\n");
    }
    println!("Displaying Perf stats:");
    println!("{}",out);
    println!("End Displaying Perf stats");

    enable_irq();
}
