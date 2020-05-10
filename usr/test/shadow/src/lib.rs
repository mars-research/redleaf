#![no_std]
extern crate malloc;
extern crate alloc;
use libsyscalls;
use syscalls::{Syscall, Heap};
use create;
use alloc::boxed::Box;
use alloc::sync::Arc;
use console::println;
use core::alloc::Layout;
use core::panic::PanicInfo;
use usr;
use rref::{RRef, RRefDeque};
use alloc::vec::Vec;
use spin::Mutex;

struct ShadowDomain {
    dom: Option<Box<dyn syscalls::Domain>>,
    dom_c: Box<dyn usr::dom_c::DomC>,
    create_dom_c: Arc<dyn create::CreateDomC>,
}

impl ShadowDomain {
    fn new(dom: Box<dyn syscalls::Domain>,
            create_dom_c: Arc<dyn create::CreateDomC>, 
            dom_c: Box<dyn usr::dom_c::DomC>) -> Self 
    {
        Self {
            dom: Some(dom),
            dom_c,
            create_dom_c,
        }
    }
}



struct Shadow {
    dom: Mutex<ShadowDomain>,
}

impl Shadow {
    fn new(dom: Box<dyn syscalls::Domain>,
            create_dom_c: Arc<dyn create::CreateDomC>, 
            dom_c: Box<dyn usr::dom_c::DomC>) -> Self 
    {
        Self {
            dom: Mutex::new(ShadowDomain::new(dom, create_dom_c, dom_c))
        }
    }
}

impl usr::dom_c::DomC for Shadow {
    fn no_arg(&self) {
        self.dom.lock().dom_c.no_arg()
    }

    fn one_arg(&self, x: usize) -> Result<usize, i64> {
        let mut dom = self.dom.lock();
        loop {
            let r = dom.dom_c.one_arg(x);
            if let Err(e) = r {

                println!("restarting domC domain");
                let old_domain = dom.dom.take();
                let (domain, dom_c) = dom.create_dom_c.recreate_domain_dom_c(old_domain.unwrap());
                dom.dom = Some(domain); 
                dom.dom_c = dom_c;

                /* restart invocation on the new domain */
                println!("restart one_arg invocation");
                continue;
            }
            break r;
        }        
    }

    fn one_rref(&self, x: RRef<usize>) -> RRef<usize> {
        self.dom.lock().dom_c.one_rref(x)
    }
}

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>, create_dom_c: Arc<dyn create::CreateDomC>) -> Box<dyn usr::dom_c::DomC> {
    libsyscalls::syscalls::init(s);
    rref::init(heap);

    println!("Init shadow domain");

    /* Create domain we're shadowing */
    let (dom, dom_c) = create_dom_c.create_domain_dom_c();

    Box::new(Shadow::new(dom, create_dom_c, dom_c))
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain shadow panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
