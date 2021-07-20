#![no_std]
#![no_main]
#![feature(
    const_fn,
    const_raw_ptr_to_usize_cast,
    untagged_unions,
    get_mut_unchecked,
    const_in_array_repeat_expressions
)]

extern crate alloc;
extern crate malloc;
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use console::println;
use core::panic::PanicInfo;
use interface::domain_create::*;
use interface::rref::RRefVec;
use libsyscalls::syscalls::{
    sys_backtrace, sys_create_thread, sys_readch_kbd, sys_recv_int, sys_yield,
};
mod bdev_wrapper;
mod nullblk;

#[cfg(feature = "test_guard_page")]
fn test_stack_exhaustion() -> u64 {
    let mut t: [u64; 4096] = [0; 4096];
    t[0] = t[1] + test_stack_exhaustion();
    t[0]
}

#[cfg(feature = "test_timer_thread")]
extern "C" fn timer_thread() {
    println!("Registering timer thread");

    loop {
        sys_recv_int(syscalls::IRQ_TIMER);
        println!("init: got a timer interrupt");
    }
}

extern "C" fn test_init_thread() {
    loop {
        println!("User init thread");
        sys_yield();
    }
}

extern "C" fn test_init_thread2() {
    loop {
        println!("User init thread 2");
        sys_yield();
    }
}

#[cfg(feature = "test_sleep")]
fn test_sleep() {
    let start = libtime::get_ns_time();
    println!("current time {}, waiting for 10_000 ms", start);

    libtime::sys_ns_sleep(10_000_000_000);

    let end = libtime::get_ns_time();
    println!(
        "current time {}, waited for {} ms",
        end,
        (end - start) / 1_000_000
    );
}

fn test_dummy_syscall() {
    use libsyscalls::syscalls::sys_dummy;
    use libtime::get_rdtsc;

    let NUM_ITER: u64 = 20_000_000;
    let start = get_rdtsc();

    for _i in 0..NUM_ITER {
        sys_dummy();
    }

    let elapsed = get_rdtsc() - start;
    println!(
        "Dummy syscall test: {} iterations took {} (avg: {} cycles)",
        NUM_ITER,
        elapsed,
        elapsed / NUM_ITER
    );
}

// AB: XXX: The following is is not supported in Rust at the moment
//
//pub fn init(s: Box<dyn syscalls::Syscall
//                    + domain_creation::CreateRv6 + domain_creation::CreateRv6FS /* + CreateRv6User */
//                    + domain_creation::CreatePCI + domain_creation::CreateAHCI + Send + Sync>)
// See
//   rustc --explain E0225
//
// We have to re-write in an ugly way
// This entry point must match with the signature in kernel/src/generated_domain_create.rs:create_domain_init
#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn syscalls::Syscall + Send + Sync>,
    heap: Box<dyn syscalls::Heap + Send + Sync>,
    ints: Box<dyn syscalls::Interrupt + Send + Sync>,
    create_proxy: Arc<dyn interface::domain_create::CreateProxy>,
    create_pci: Arc<dyn interface::domain_create::CreatePCI>,
    create_membdev: Arc<dyn interface::domain_create::CreateMemBDev>,
    create_bdev_shadow: Arc<dyn interface::domain_create::CreateBDevShadow>,
    create_ixgbe: Arc<dyn interface::domain_create::CreateIxgbe>,
    create_virtio_net: Arc<dyn interface::domain_create::CreateVirtioNet>,
    create_virtio_block: Arc<dyn interface::domain_create::CreateVirtioBlock>,
    create_net_shadow: Arc<dyn interface::domain_create::CreateNetShadow>,
    create_nvme_shadow: Arc<dyn interface::domain_create::CreateNvmeShadow>,
    create_nvme: Arc<dyn interface::domain_create::CreateNvme>,
    create_xv6fs: Arc<dyn interface::domain_create::CreateRv6FS>,
    create_xv6net: Arc<dyn interface::domain_create::CreateRv6Net>,
    create_xv6net_shadow: Arc<dyn interface::domain_create::CreateRv6NetShadow>,
    create_xv6usr: Arc<dyn interface::domain_create::CreateRv6Usr>,
    create_xv6: Arc<dyn interface::domain_create::CreateRv6>,
    create_dom_c: Arc<dyn interface::domain_create::CreateDomC>,
    create_dom_d: Arc<dyn interface::domain_create::CreateDomD>,
    create_shadow: Arc<dyn interface::domain_create::CreateShadow>,
    create_benchnvme: Arc<dyn interface::domain_create::CreateBenchnvme>,
    create_tpm: Arc<dyn interface::domain_create::CreateTpm>,
) {
    libsyscalls::syscalls::init(s);
    interface::rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    let ints_clone = ints.int_clone();
    libsyscalls::syscalls::init_interrupts(ints);

    //let b = Box::new(4);
    //let r = sys_alloc();
    let mut v1: Vec<u64> = Vec::with_capacity(1024);
    for i in 0..2048 {
        v1.push(i);
    }

    println!("{} {} {}", "init", "userland", 1);

    //println!("init userland print works");

    #[cfg(feature = "test_guard_page")]
    {
        println!("start the test guard page test");
        let foo = test_stack_exhaustion();
        println!("test guard page: {}", foo);
    }

    #[cfg(feature = "test_timer_thread")]
    {
        let t = sys_create_thread("init_int[timer]", timer_thread);
        t.set_priority(10);
    }

    #[cfg(feature = "test_sleep")]
    test_sleep();

    #[cfg(feature = "test_threads")]
    {
        let t = sys_create_thread("init_thread", test_init_thread);
        t.set_affinity(1);

        let t2 = sys_create_thread("init_thread_2", test_init_thread2);
        t2.set_affinity(0);

        #[cfg(feature = "test_sleep")]
        test_sleep();

        println!("Setting affinity to CPUs 2 and 3");
        t.set_affinity(2);
        t2.set_affinity(3);

        #[cfg(feature = "test_sleep")]
        test_sleep();

        println!("Setting affinity to CPUs 1 and 1");
        t.set_affinity(1);
        t2.set_affinity(1);

        drop(t);
        drop(t2);
    }

    // test_dummy_syscall();

    println!("about to create proxy");
    let (_dom_proxy, proxy) = create_proxy.create_domain_proxy(
        create_pci,
        // create_ahci,
        create_membdev,
        create_bdev_shadow,
        create_ixgbe,
        create_virtio_net,
        create_virtio_block,
        create_nvme,
        create_net_shadow,
        create_nvme_shadow,
        // create_benchnet,
        create_benchnvme,
        create_xv6fs,
        create_xv6net,
        create_xv6net_shadow,
        create_xv6usr,
        create_xv6,
        create_dom_c,
        create_dom_d,
        create_shadow,
        create_tpm,
    );
    println!("created proxy");

    #[cfg(feature = "test_cd")]
    {
        #[cfg(not(feature = "shadow"))]
        let (dom_dom_c, dom_c) = proxy.as_domain_create_CreateDomC().create_domain_dom_c();
        #[cfg(feature = "shadow")]
        let (dom_shadow, dom_c) = proxy
            .as_create_shadow()
            .create_domain_shadow(proxy.as_create_dom_c());
        let dom_dom_d = proxy
            .as_domain_create_CreateDomD()
            .create_domain_dom_d(dom_c);
    }

    #[cfg(feature = "tpm")]
    let (_dom_tpm, usr_tpm) = proxy.as_domain_create_CreateTpm().create_domain_tpm();

    #[cfg(feature = "hashbench")]
    let dom_hashstore = proxy
        .as_domain_create_CreateSashstore()
        .create_domain_hashstore();

    println!("Creating pci");
    let (_dom_pci, pci) = proxy.as_domain_create_CreatePCI().create_domain_pci();

    #[cfg(feature = "virtio_net")]
    let (_, net) = proxy
        .as_domain_create_CreateVirtioNet()
        .create_domain_virtio_net(pci.pci_clone().unwrap());
    #[cfg(all(not(feature = "shadow"), not(feature = "virtnet")))]
    let (_, net) = proxy.as_create_ixgbe().create_domain_ixgbe(pci.pci_clone());
    #[cfg(all(feature = "shadow", not(feature = "virtio_net")))]
    let (_, net) = proxy
        .as_create_net_shadow()
        .create_domain_net_shadow(proxy.as_create_ixgbe(), pci.pci_clone());

    #[cfg(not(feature = "membdev"))]
    let (dom_ahci, bdev) = proxy.as_create_ahci().create_domain_ahci(pci.pci_clone());

    #[cfg(feature = "membdev")]
    #[cfg(not(feature = "shadow"))]
    // Memfs is linked with the shadow domain so membdev doesn't work without shadow currently.
    let (dom_ahci, bdev) = proxy
        .as_domain_create_CreateMemBDev()
        .create_domain_membdev(&mut []);
    #[cfg(feature = "membdev")]
    #[cfg(feature = "shadow")]
    let (_dom_ahci, bdev) = proxy
        .as_domain_create_CreateBDevShadow()
        .create_domain_bdev_shadow(proxy.as_domain_create_CreateMemBDev());

    println!("Creating nvme domain!");
    #[cfg(not(feature = "shadow"))]
    let (dom_nvme, nvme) = proxy
        .as_domain_create_CreateNvme()
        .create_domain_nvme(pci.pci_clone().unwrap());
    #[cfg(feature = "shadow")]
    let (_dom_nvme, nvme) = proxy
        .as_domain_create_CreateNvmeShadow()
        .create_domain_nvme_shadow(
            proxy.as_domain_create_CreateNvme(),
            pci.pci_clone().unwrap(),
        );

    println!("Creating ixgbe");
    #[cfg(not(feature = "shadow"))]
    let (dom_ixgbe, net) = proxy
        .as_domain_create_CreateIxgbe()
        .create_domain_ixgbe(pci.pci_clone().unwrap());
    #[cfg(feature = "shadow")]
    let (_dom_ixgbe, net) = proxy
        .as_domain_create_CreateNetShadow()
        .create_domain_net_shadow(
            proxy.as_domain_create_CreateIxgbe(),
            pci.pci_clone().unwrap(),
        );

    #[cfg(feature = "benchnet")]
    let _ = proxy.as_create_benchnet().create_domain_benchnet(net);

    #[cfg(feature = "benchnvme")]
    let _ = proxy.as_create_benchnvme().create_domain_benchnvme(nvme);

    #[cfg(feature = "virtio_block")]
    let (_, nvme) = proxy
        .as_domain_create_CreateVirtioBlock()
        .create_domain_virtio_block(pci.pci_clone().unwrap());

    #[cfg(not(any(feature = "benchnet", feature = "benchnvme")))]
    {
        println!("Starting xv6 kernel");
        let (_dom_xv6, rv6) = proxy.as_domain_create_CreateRv6().create_domain_xv6kernel(
            ints_clone,
            proxy.as_domain_create_CreateRv6FS(),
            proxy.as_domain_create_CreateRv6Net(),
            proxy.as_domain_create_CreateRv6NetShadow(),
            proxy.as_domain_create_CreateRv6Usr(),
            // bdev,
            Box::new(bdev_wrapper::BDevWrapper::new(nvme)),
            net,
            Box::new(nullblk::NullBlk::new()),
            usr_tpm,
        );
        println!("Starting xv6 user init");
        rv6.sys_spawn_domain(
            rv6.clone_rv6().unwrap(),
            RRefVec::from_slice("/init".as_bytes()),
            RRefVec::from_slice("/init".as_bytes()),
            array_init::array_init(|_| None),
        )
        .unwrap()
        .unwrap();
    }
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("redleaf init panic: {:?}", info);
    sys_backtrace();
    loop {}
}
