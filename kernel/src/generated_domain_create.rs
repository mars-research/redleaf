impl interface::domain_create::CreateProxy for crate::syscalls::PDomain {
    fn create_domain_proxy(
        &self,
        create_pci: alloc::sync::Arc<dyn interface::domain_create::CreatePCI>,
        create_membdev: alloc::sync::Arc<dyn interface::domain_create::CreateMemBDev>,
        create_bdev_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateBDevShadow>,
        create_ixgbe: alloc::sync::Arc<dyn interface::domain_create::CreateIxgbe>,
        create_virtio_net: alloc::sync::Arc<dyn interface::domain_create::CreateVirtioNet>,
        create_virtio_block: alloc::sync::Arc<dyn interface::domain_create::CreateVirtioBlock>,
        create_nvme: alloc::sync::Arc<dyn interface::domain_create::CreateNvme>,
        create_net_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateNetShadow>,
        create_nvme_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateNvmeShadow>,
        create_benchnvme: alloc::sync::Arc<dyn interface::domain_create::CreateBenchnvme>,
        create_xv6fs: alloc::sync::Arc<dyn interface::domain_create::CreateRv6FS>,
        create_xv6net: alloc::sync::Arc<dyn interface::domain_create::CreateRv6Net>,
        create_xv6net_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateRv6NetShadow>,
        create_xv6usr: alloc::sync::Arc<dyn interface::domain_create::CreateRv6Usr>,
        create_xv6: alloc::sync::Arc<dyn interface::domain_create::CreateRv6>,
        create_dom_c: alloc::sync::Arc<dyn interface::domain_create::CreateDomC>,
        create_dom_d: alloc::sync::Arc<dyn interface::domain_create::CreateDomD>,
        create_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateShadow>,
        create_tpm: alloc::sync::Arc<dyn interface::domain_create::CreateTpm>,
    ) -> (
        alloc::boxed::Box<dyn syscalls::Domain>,
        alloc::sync::Arc<dyn interface::proxy::Proxy>,
    ) {
        crate::interrupt::disable_irq();
        let rtn_ = dom_proxy_create_domain_proxy(
            create_pci: alloc::sync::Arc<dyn interface::domain_create::CreatePCI>,
            create_membdev: alloc::sync::Arc<dyn interface::domain_create::CreateMemBDev>,
            create_bdev_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateBDevShadow>,
            create_ixgbe: alloc::sync::Arc<dyn interface::domain_create::CreateIxgbe>,
            create_virtio_net: alloc::sync::Arc<dyn interface::domain_create::CreateVirtioNet>,
            create_virtio_block: alloc::sync::Arc<dyn interface::domain_create::CreateVirtioBlock>,
            create_nvme: alloc::sync::Arc<dyn interface::domain_create::CreateNvme>,
            create_net_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateNetShadow>,
            create_nvme_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateNvmeShadow>,
            create_benchnvme: alloc::sync::Arc<dyn interface::domain_create::CreateBenchnvme>,
            create_xv6fs: alloc::sync::Arc<dyn interface::domain_create::CreateRv6FS>,
            create_xv6net: alloc::sync::Arc<dyn interface::domain_create::CreateRv6Net>,
            create_xv6net_shadow:
                alloc::sync::Arc<dyn interface::domain_create::CreateRv6NetShadow>,
            create_xv6usr: alloc::sync::Arc<dyn interface::domain_create::CreateRv6Usr>,
            create_xv6: alloc::sync::Arc<dyn interface::domain_create::CreateRv6>,
            create_dom_c: alloc::sync::Arc<dyn interface::domain_create::CreateDomC>,
            create_dom_d: alloc::sync::Arc<dyn interface::domain_create::CreateDomD>,
            create_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateShadow>,
            create_tpm: alloc::sync::Arc<dyn interface::domain_create::CreateTpm>,
        );
        crate::interrupt::enable_irq();
        rtn_
    }
}
pub(crate) fn dom_proxy_create_domain_proxy(
    create_pci: alloc::sync::Arc<dyn interface::domain_create::CreatePCI>,
    create_membdev: alloc::sync::Arc<dyn interface::domain_create::CreateMemBDev>,
    create_bdev_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateBDevShadow>,
    create_ixgbe: alloc::sync::Arc<dyn interface::domain_create::CreateIxgbe>,
    create_virtio_net: alloc::sync::Arc<dyn interface::domain_create::CreateVirtioNet>,
    create_virtio_block: alloc::sync::Arc<dyn interface::domain_create::CreateVirtioBlock>,
    create_nvme: alloc::sync::Arc<dyn interface::domain_create::CreateNvme>,
    create_net_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateNetShadow>,
    create_nvme_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateNvmeShadow>,
    create_benchnvme: alloc::sync::Arc<dyn interface::domain_create::CreateBenchnvme>,
    create_xv6fs: alloc::sync::Arc<dyn interface::domain_create::CreateRv6FS>,
    create_xv6net: alloc::sync::Arc<dyn interface::domain_create::CreateRv6Net>,
    create_xv6net_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateRv6NetShadow>,
    create_xv6usr: alloc::sync::Arc<dyn interface::domain_create::CreateRv6Usr>,
    create_xv6: alloc::sync::Arc<dyn interface::domain_create::CreateRv6>,
    create_dom_c: alloc::sync::Arc<dyn interface::domain_create::CreateDomC>,
    create_dom_d: alloc::sync::Arc<dyn interface::domain_create::CreateDomD>,
    create_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateShadow>,
    create_tpm: alloc::sync::Arc<dyn interface::domain_create::CreateTpm>,
) -> (
    alloc::boxed::Box<dyn syscalls::Domain>,
    alloc::sync::Arc<dyn interface::proxy::Proxy>,
) {
    crate::interrupt::disable_irq();
    extern "C" {
        fn _binary_domains_build_dom_proxy_start();
        fn _binary_domains_build_dom_proxy_end();
    }
    let binary_range_ = (
        _binary_domains_build_dom_proxy_start as *const u8,
        _binary_domains_build_dom_proxy_end as *const u8,
    );
    type UserInit_ = fn(
        ::alloc::boxed::Box<dyn ::syscalls::Syscall>,
        ::alloc::boxed::Box<dyn ::syscalls::Heap>,
        create_pci: alloc::sync::Arc<dyn interface::domain_create::CreatePCI>,
        create_membdev: alloc::sync::Arc<dyn interface::domain_create::CreateMemBDev>,
        create_bdev_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateBDevShadow>,
        create_ixgbe: alloc::sync::Arc<dyn interface::domain_create::CreateIxgbe>,
        create_virtio_net: alloc::sync::Arc<dyn interface::domain_create::CreateVirtioNet>,
        create_virtio_block: alloc::sync::Arc<dyn interface::domain_create::CreateVirtioBlock>,
        create_nvme: alloc::sync::Arc<dyn interface::domain_create::CreateNvme>,
        create_net_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateNetShadow>,
        create_nvme_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateNvmeShadow>,
        create_benchnvme: alloc::sync::Arc<dyn interface::domain_create::CreateBenchnvme>,
        create_xv6fs: alloc::sync::Arc<dyn interface::domain_create::CreateRv6FS>,
        create_xv6net: alloc::sync::Arc<dyn interface::domain_create::CreateRv6Net>,
        create_xv6net_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateRv6NetShadow>,
        create_xv6usr: alloc::sync::Arc<dyn interface::domain_create::CreateRv6Usr>,
        create_xv6: alloc::sync::Arc<dyn interface::domain_create::CreateRv6>,
        create_dom_c: alloc::sync::Arc<dyn interface::domain_create::CreateDomC>,
        create_dom_d: alloc::sync::Arc<dyn interface::domain_create::CreateDomD>,
        create_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateShadow>,
        create_tpm: alloc::sync::Arc<dyn interface::domain_create::CreateTpm>,
    ) -> alloc::sync::Arc<dyn interface::proxy::Proxy>;
    let (dom_, entry_) = unsafe { crate::domain::load_domain("dom_proxy", binary_range_) };
    let user_ep_: UserInit_ = unsafe { ::core::mem::transmute::<*const (), UserInit_>(entry_) };
    let pdom_ = ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
        &dom_,
    )));
    let pheap_ = ::alloc::boxed::Box::new(crate::heap::PHeap::new());
    let thread_ = crate::thread::get_current_ref();
    let old_id_ = {
        let mut thread = thread_.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom_.lock().id;
        old_id
    };
    crate::interrupt::enable_irq();
    let ep_rtn_ = user_ep_(
        pdom_,
        pheap_,
        create_pci: alloc::sync::Arc<dyn interface::domain_create::CreatePCI>,
        create_membdev: alloc::sync::Arc<dyn interface::domain_create::CreateMemBDev>,
        create_bdev_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateBDevShadow>,
        create_ixgbe: alloc::sync::Arc<dyn interface::domain_create::CreateIxgbe>,
        create_virtio_net: alloc::sync::Arc<dyn interface::domain_create::CreateVirtioNet>,
        create_virtio_block: alloc::sync::Arc<dyn interface::domain_create::CreateVirtioBlock>,
        create_nvme: alloc::sync::Arc<dyn interface::domain_create::CreateNvme>,
        create_net_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateNetShadow>,
        create_nvme_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateNvmeShadow>,
        create_benchnvme: alloc::sync::Arc<dyn interface::domain_create::CreateBenchnvme>,
        create_xv6fs: alloc::sync::Arc<dyn interface::domain_create::CreateRv6FS>,
        create_xv6net: alloc::sync::Arc<dyn interface::domain_create::CreateRv6Net>,
        create_xv6net_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateRv6NetShadow>,
        create_xv6usr: alloc::sync::Arc<dyn interface::domain_create::CreateRv6Usr>,
        create_xv6: alloc::sync::Arc<dyn interface::domain_create::CreateRv6>,
        create_dom_c: alloc::sync::Arc<dyn interface::domain_create::CreateDomC>,
        create_dom_d: alloc::sync::Arc<dyn interface::domain_create::CreateDomD>,
        create_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateShadow>,
        create_tpm: alloc::sync::Arc<dyn interface::domain_create::CreateTpm>,
    );
    crate::interrupt::disable_irq();
    {
        thread_.lock().current_domain_id = old_id_;
    }
    #[cfg(feature = "domain_create_log")]
    println!("domain/{}: returned from entry point", "dom_proxy");
    let dom_: ::alloc::boxed::Box<dyn ::syscalls::Domain> = ::alloc::boxed::Box::new(
        crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom_)),
    );
    let rtn_ = (dom_, ep_rtn_);
    crate::interrupt::enable_irq();
    rtn_
}
impl interface::domain_create::CreatePCI for crate::syscalls::PDomain {
    fn create_domain_pci(
        &self,
    ) -> (
        alloc::boxed::Box<dyn syscalls::Domain>,
        alloc::boxed::Box<dyn interface::pci::PCI>,
    ) {
        crate::interrupt::disable_irq();
        let rtn_ = pci_create_domain_pci();
        crate::interrupt::enable_irq();
        rtn_
    }
}
pub(crate) fn pci_create_domain_pci() -> (
    alloc::boxed::Box<dyn syscalls::Domain>,
    alloc::boxed::Box<dyn interface::pci::PCI>,
) {
    crate::interrupt::disable_irq();
    extern "C" {
        fn _binary_domains_build_pci_start();
        fn _binary_domains_build_pci_end();
    }
    let binary_range_ = (
        _binary_domains_build_pci_start as *const u8,
        _binary_domains_build_pci_end as *const u8,
    );
    type UserInit_ = fn(
        ::alloc::boxed::Box<dyn ::syscalls::Syscall>,
        ::alloc::boxed::Box<dyn ::syscalls::Mmap>,
        ::alloc::boxed::Box<dyn ::syscalls::Heap>,
    ) -> alloc::boxed::Box<dyn interface::pci::PCI>;
    let (dom_, entry_) = unsafe { crate::domain::load_domain("pci", binary_range_) };
    let user_ep_: UserInit_ = unsafe { ::core::mem::transmute::<*const (), UserInit_>(entry_) };

    let pdom_ = ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
        &dom_,
    )));
    let pmmap_ = ::alloc::boxed::Box::new(crate::syscalls::Mmap::new());
    let pheap_ = ::alloc::boxed::Box::new(crate::heap::PHeap::new());
    let thread_ = crate::thread::get_current_ref();
    let old_id_ = {
        let mut thread = thread_.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom_.lock().id;
        old_id
    };
    crate::interrupt::enable_irq();
    let ep_rtn_ = user_ep_(pdom_, pmmap_, pheap_);
    crate::interrupt::disable_irq();
    {
        thread_.lock().current_domain_id = old_id_;
    }
    #[cfg(feature = "domain_create_log")]
    println!("domain/{}: returned from entry point", "pci");
    let dom_: ::alloc::boxed::Box<dyn ::syscalls::Domain> = ::alloc::boxed::Box::new(
        crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom_)),
    );
    let rtn_ = (dom_, ep_rtn_);
    crate::interrupt::enable_irq();
    rtn_
}
impl interface::domain_create::CreateMemBDev for crate::syscalls::PDomain {
    fn create_domain_membdev(
        &self,
        memdisk: &'static mut [u8],
    ) -> (
        alloc::boxed::Box<dyn syscalls::Domain>,
        alloc::boxed::Box<dyn interface::bdev::BDev>,
    ) {
        crate::interrupt::disable_irq();
        let rtn_ = membdev_create_domain_membdev(memdisk: &'static mut [u8]);
        crate::interrupt::enable_irq();
        rtn_
    }
    fn recreate_domain_membdev(
        &self,
        dom: alloc::boxed::Box<dyn syscalls::Domain>,
        memdisk: &'static mut [u8],
    ) -> (
        alloc::boxed::Box<dyn syscalls::Domain>,
        alloc::boxed::Box<dyn interface::bdev::BDev>,
    ) {
        crate::interrupt::disable_irq();
        let rtn_ = membdev_recreate_domain_membdev(
            dom: alloc::boxed::Box<dyn syscalls::Domain>,
            memdisk: &'static mut [u8],
        );
        crate::interrupt::enable_irq();
        rtn_
    }
}
pub(crate) fn membdev_create_domain_membdev(
    memdisk: &'static mut [u8],
) -> (
    alloc::boxed::Box<dyn syscalls::Domain>,
    alloc::boxed::Box<dyn interface::bdev::BDev>,
) {
    crate::interrupt::disable_irq();
    extern "C" {
        fn _binary_domains_build_membdev_start();
        fn _binary_domains_build_membdev_end();
    }
    let binary_range_ = (
        _binary_domains_build_membdev_start as *const u8,
        _binary_domains_build_membdev_end as *const u8,
    );
    type UserInit_ = fn(
        ::alloc::boxed::Box<dyn ::syscalls::Syscall>,
        ::alloc::boxed::Box<dyn ::syscalls::Heap>,
        memdisk: &'static mut [u8],
    ) -> alloc::boxed::Box<dyn interface::bdev::BDev>;
    let (dom_, entry_) = unsafe { crate::domain::load_domain("membdev", binary_range_) };
    let user_ep_: UserInit_ = unsafe { ::core::mem::transmute::<*const (), UserInit_>(entry_) };
    let pdom_ = ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
        &dom_,
    )));
    let pheap_ = ::alloc::boxed::Box::new(crate::heap::PHeap::new());
    let thread_ = crate::thread::get_current_ref();
    let old_id_ = {
        let mut thread = thread_.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom_.lock().id;
        old_id
    };
    crate::interrupt::enable_irq();
    let ep_rtn_ = user_ep_(pdom_, pheap_, memdisk: &'static mut [u8]);
    crate::interrupt::disable_irq();
    {
        thread_.lock().current_domain_id = old_id_;
    }
    #[cfg(feature = "domain_create_log")]
    println!("domain/{}: returned from entry point", "membdev");
    let dom_: ::alloc::boxed::Box<dyn ::syscalls::Domain> = ::alloc::boxed::Box::new(
        crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom_)),
    );
    let rtn_ = (dom_, ep_rtn_);
    crate::interrupt::enable_irq();
    rtn_
}
pub(crate) fn membdev_recreate_domain_membdev(
    dom: alloc::boxed::Box<dyn syscalls::Domain>,
    memdisk: &'static mut [u8],
) -> (
    alloc::boxed::Box<dyn syscalls::Domain>,
    alloc::boxed::Box<dyn interface::bdev::BDev>,
) {
    crate::interrupt::disable_irq();
    extern "C" {
        fn _binary_domains_build_membdev_start();
        fn _binary_domains_build_membdev_end();
    }
    let binary_range_ = (
        _binary_domains_build_membdev_start as *const u8,
        _binary_domains_build_membdev_end as *const u8,
    );
    type UserInit_ = fn(
        ::alloc::boxed::Box<dyn ::syscalls::Syscall>,
        ::alloc::boxed::Box<dyn ::syscalls::Heap>,
        dom: alloc::boxed::Box<dyn syscalls::Domain>,
        memdisk: &'static mut [u8],
    ) -> alloc::boxed::Box<dyn interface::bdev::BDev>;
    let (dom_, entry_) = unsafe { crate::domain::load_domain("membdev", binary_range_) };
    let user_ep_: UserInit_ = unsafe { ::core::mem::transmute::<*const (), UserInit_>(entry_) };
    let pdom_ = ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
        &dom_,
    )));
    let pheap_ = ::alloc::boxed::Box::new(crate::heap::PHeap::new());
    let thread_ = crate::thread::get_current_ref();
    let old_id_ = {
        let mut thread = thread_.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom_.lock().id;
        old_id
    };
    crate::interrupt::enable_irq();
    let ep_rtn_ = user_ep_(
        pdom_,
        pheap_,
        dom: alloc::boxed::Box<dyn syscalls::Domain>,
        memdisk: &'static mut [u8],
    );
    crate::interrupt::disable_irq();
    {
        thread_.lock().current_domain_id = old_id_;
    }
    #[cfg(feature = "domain_create_log")]
    println!("domain/{}: returned from entry point", "membdev");
    let dom_: ::alloc::boxed::Box<dyn ::syscalls::Domain> = ::alloc::boxed::Box::new(
        crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom_)),
    );
    let rtn_ = (dom_, ep_rtn_);
    crate::interrupt::enable_irq();
    rtn_
}
impl interface::domain_create::CreateBDevShadow for crate::syscalls::PDomain {
    fn create_domain_bdev_shadow(
        &self,
        create: alloc::sync::Arc<dyn interface::domain_create::CreateMemBDev>,
    ) -> (
        alloc::boxed::Box<dyn syscalls::Domain>,
        alloc::boxed::Box<dyn interface::bdev::BDev>,
    ) {
        crate::interrupt::disable_irq();
        let rtn_ = bdev_shadow_create_domain_bdev_shadow(
            create: alloc::sync::Arc<dyn interface::domain_create::CreateMemBDev>,
        );
        crate::interrupt::enable_irq();
        rtn_
    }
}
pub(crate) fn bdev_shadow_create_domain_bdev_shadow(
    create: alloc::sync::Arc<dyn interface::domain_create::CreateMemBDev>,
) -> (
    alloc::boxed::Box<dyn syscalls::Domain>,
    alloc::boxed::Box<dyn interface::bdev::BDev>,
) {
    crate::interrupt::disable_irq();
    extern "C" {
        fn _binary_domains_build_bdev_shadow_start();
        fn _binary_domains_build_bdev_shadow_end();
    }
    let binary_range_ = (
        _binary_domains_build_bdev_shadow_start as *const u8,
        _binary_domains_build_bdev_shadow_end as *const u8,
    );
    type UserInit_ = fn(
        ::alloc::boxed::Box<dyn ::syscalls::Syscall>,
        ::alloc::boxed::Box<dyn ::syscalls::Heap>,
        create: alloc::sync::Arc<dyn interface::domain_create::CreateMemBDev>,
    ) -> alloc::boxed::Box<dyn interface::bdev::BDev>;
    let (dom_, entry_) = unsafe { crate::domain::load_domain("bdev_shadow", binary_range_) };
    let user_ep_: UserInit_ = unsafe { ::core::mem::transmute::<*const (), UserInit_>(entry_) };
    let pdom_ = ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
        &dom_,
    )));
    let pheap_ = ::alloc::boxed::Box::new(crate::heap::PHeap::new());
    let thread_ = crate::thread::get_current_ref();
    let old_id_ = {
        let mut thread = thread_.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom_.lock().id;
        old_id
    };
    crate::interrupt::enable_irq();
    let ep_rtn_ = user_ep_(
        pdom_,
        pheap_,
        create: alloc::sync::Arc<dyn interface::domain_create::CreateMemBDev>,
    );
    crate::interrupt::disable_irq();
    {
        thread_.lock().current_domain_id = old_id_;
    }
    #[cfg(feature = "domain_create_log")]
    println!("domain/{}: returned from entry point", "bdev_shadow");
    let dom_: ::alloc::boxed::Box<dyn ::syscalls::Domain> = ::alloc::boxed::Box::new(
        crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom_)),
    );
    let rtn_ = (dom_, ep_rtn_);
    crate::interrupt::enable_irq();
    rtn_
}
impl interface::domain_create::CreateIxgbe for crate::syscalls::PDomain {
    fn create_domain_ixgbe(
        &self,
        pci: alloc::boxed::Box<dyn interface::pci::PCI>,
    ) -> (
        alloc::boxed::Box<dyn syscalls::Domain>,
        alloc::boxed::Box<dyn interface::net::Net>,
    ) {
        crate::interrupt::disable_irq();
        let rtn_ = ixgbe_create_domain_ixgbe(pci: alloc::boxed::Box<dyn interface::pci::PCI>);
        crate::interrupt::enable_irq();
        rtn_
    }
}
pub(crate) fn ixgbe_create_domain_ixgbe(
    pci: alloc::boxed::Box<dyn interface::pci::PCI>,
) -> (
    alloc::boxed::Box<dyn syscalls::Domain>,
    alloc::boxed::Box<dyn interface::net::Net>,
) {
    crate::interrupt::disable_irq();
    extern "C" {
        fn _binary_domains_build_ixgbe_start();
        fn _binary_domains_build_ixgbe_end();
    }
    let binary_range_ = (
        _binary_domains_build_ixgbe_start as *const u8,
        _binary_domains_build_ixgbe_end as *const u8,
    );
    type UserInit_ = fn(
        ::alloc::boxed::Box<dyn ::syscalls::Syscall>,
        ::alloc::boxed::Box<dyn ::syscalls::Heap>,
        pci: alloc::boxed::Box<dyn interface::pci::PCI>,
    ) -> alloc::boxed::Box<dyn interface::net::Net>;
    let (dom_, entry_) = unsafe { crate::domain::load_domain("ixgbe", binary_range_) };
    let user_ep_: UserInit_ = unsafe { ::core::mem::transmute::<*const (), UserInit_>(entry_) };
    let pdom_ = ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
        &dom_,
    )));
    let pheap_ = ::alloc::boxed::Box::new(crate::heap::PHeap::new());
    let thread_ = crate::thread::get_current_ref();
    let old_id_ = {
        let mut thread = thread_.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom_.lock().id;
        old_id
    };
    crate::interrupt::enable_irq();
    let ep_rtn_ = user_ep_(
        pdom_,
        pheap_,
        pci: alloc::boxed::Box<dyn interface::pci::PCI>,
    );
    crate::interrupt::disable_irq();
    {
        thread_.lock().current_domain_id = old_id_;
    }
    #[cfg(feature = "domain_create_log")]
    println!("domain/{}: returned from entry point", "ixgbe");
    let dom_: ::alloc::boxed::Box<dyn ::syscalls::Domain> = ::alloc::boxed::Box::new(
        crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom_)),
    );
    let rtn_ = (dom_, ep_rtn_);
    crate::interrupt::enable_irq();
    rtn_
}
impl interface::domain_create::CreateVirtioNet for crate::syscalls::PDomain {
    fn create_domain_virtio_net(
        &self,
        pci: alloc::boxed::Box<dyn interface::pci::PCI>,
    ) -> (
        alloc::boxed::Box<dyn syscalls::Domain>,
        alloc::boxed::Box<dyn interface::net::Net>,
    ) {
        crate::interrupt::disable_irq();
        let rtn_ =
            virtio_net_create_domain_virtio_net(pci: alloc::boxed::Box<dyn interface::pci::PCI>);
        crate::interrupt::enable_irq();
        rtn_
    }
}
pub(crate) fn virtio_net_create_domain_virtio_net(
    pci: alloc::boxed::Box<dyn interface::pci::PCI>,
) -> (
    alloc::boxed::Box<dyn syscalls::Domain>,
    alloc::boxed::Box<dyn interface::net::Net>,
) {
    crate::interrupt::disable_irq();
    extern "C" {
        fn _binary_domains_build_virtio_net_start();
        fn _binary_domains_build_virtio_net_end();
    }
    let binary_range_ = (
        _binary_domains_build_virtio_net_start as *const u8,
        _binary_domains_build_virtio_net_end as *const u8,
    );
    type UserInit_ = fn(
        ::alloc::boxed::Box<dyn ::syscalls::Syscall>,
        ::alloc::boxed::Box<dyn ::syscalls::Heap>,
        pci: alloc::boxed::Box<dyn interface::pci::PCI>,
    ) -> alloc::boxed::Box<dyn interface::net::Net>;
    let (dom_, entry_) = unsafe { crate::domain::load_domain("virtio_net", binary_range_) };
    let user_ep_: UserInit_ = unsafe { ::core::mem::transmute::<*const (), UserInit_>(entry_) };
    let pdom_ = ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
        &dom_,
    )));
    let pheap_ = ::alloc::boxed::Box::new(crate::heap::PHeap::new());
    let thread_ = crate::thread::get_current_ref();
    let old_id_ = {
        let mut thread = thread_.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom_.lock().id;
        old_id
    };
    crate::interrupt::enable_irq();
    let ep_rtn_ = user_ep_(
        pdom_,
        pheap_,
        pci: alloc::boxed::Box<dyn interface::pci::PCI>,
    );
    crate::interrupt::disable_irq();
    {
        thread_.lock().current_domain_id = old_id_;
    }
    #[cfg(feature = "domain_create_log")]
    println!("domain/{}: returned from entry point", "virtio_net");
    let dom_: ::alloc::boxed::Box<dyn ::syscalls::Domain> = ::alloc::boxed::Box::new(
        crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom_)),
    );
    let rtn_ = (dom_, ep_rtn_);
    crate::interrupt::enable_irq();
    rtn_
}
impl interface::domain_create::CreateVirtioBlock for crate::syscalls::PDomain {
    fn create_domain_virtio_block(
        &self,
        pci: alloc::boxed::Box<dyn interface::pci::PCI>,
    ) -> (
        alloc::boxed::Box<dyn syscalls::Domain>,
        alloc::boxed::Box<dyn interface::bdev::NvmeBDev>,
    ) {
        crate::interrupt::disable_irq();
        let rtn_ = virtio_block_create_domain_virtio_block(
            pci: alloc::boxed::Box<dyn interface::pci::PCI>,
        );
        crate::interrupt::enable_irq();
        rtn_
    }
}
pub(crate) fn virtio_block_create_domain_virtio_block(
    pci: alloc::boxed::Box<dyn interface::pci::PCI>,
) -> (
    alloc::boxed::Box<dyn syscalls::Domain>,
    alloc::boxed::Box<dyn interface::bdev::NvmeBDev>,
) {
    crate::interrupt::disable_irq();
    extern "C" {
        fn _binary_domains_build_virtio_block_start();
        fn _binary_domains_build_virtio_block_end();
    }
    let binary_range_ = (
        _binary_domains_build_virtio_block_start as *const u8,
        _binary_domains_build_virtio_block_end as *const u8,
    );
    type UserInit_ = fn(
        ::alloc::boxed::Box<dyn ::syscalls::Syscall>,
        ::alloc::boxed::Box<dyn ::syscalls::Heap>,
        pci: alloc::boxed::Box<dyn interface::pci::PCI>,
    ) -> alloc::boxed::Box<dyn interface::bdev::NvmeBDev>;
    let (dom_, entry_) = unsafe { crate::domain::load_domain("virtio_block", binary_range_) };
    let user_ep_: UserInit_ = unsafe { ::core::mem::transmute::<*const (), UserInit_>(entry_) };
    let pdom_ = ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
        &dom_,
    )));
    let pheap_ = ::alloc::boxed::Box::new(crate::heap::PHeap::new());
    let thread_ = crate::thread::get_current_ref();
    let old_id_ = {
        let mut thread = thread_.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom_.lock().id;
        old_id
    };
    crate::interrupt::enable_irq();
    let ep_rtn_ = user_ep_(
        pdom_,
        pheap_,
        pci: alloc::boxed::Box<dyn interface::pci::PCI>,
    );
    crate::interrupt::disable_irq();
    {
        thread_.lock().current_domain_id = old_id_;
    }
    #[cfg(feature = "domain_create_log")]
    println!("domain/{}: returned from entry point", "virtio_block");
    let dom_: ::alloc::boxed::Box<dyn ::syscalls::Domain> = ::alloc::boxed::Box::new(
        crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom_)),
    );
    let rtn_ = (dom_, ep_rtn_);
    crate::interrupt::enable_irq();
    rtn_
}
impl interface::domain_create::CreateNetShadow for crate::syscalls::PDomain {
    fn create_domain_net_shadow(
        &self,
        create: alloc::sync::Arc<dyn interface::domain_create::CreateIxgbe>,
        pci: alloc::boxed::Box<dyn interface::pci::PCI>,
    ) -> (
        alloc::boxed::Box<dyn syscalls::Domain>,
        alloc::boxed::Box<dyn interface::net::Net>,
    ) {
        crate::interrupt::disable_irq();
        let rtn_ = net_shadow_create_domain_net_shadow(
            create: alloc::sync::Arc<dyn interface::domain_create::CreateIxgbe>,
            pci: alloc::boxed::Box<dyn interface::pci::PCI>,
        );
        crate::interrupt::enable_irq();
        rtn_
    }
}
pub(crate) fn net_shadow_create_domain_net_shadow(
    create: alloc::sync::Arc<dyn interface::domain_create::CreateIxgbe>,
    pci: alloc::boxed::Box<dyn interface::pci::PCI>,
) -> (
    alloc::boxed::Box<dyn syscalls::Domain>,
    alloc::boxed::Box<dyn interface::net::Net>,
) {
    crate::interrupt::disable_irq();
    extern "C" {
        fn _binary_domains_build_net_shadow_start();
        fn _binary_domains_build_net_shadow_end();
    }
    let binary_range_ = (
        _binary_domains_build_net_shadow_start as *const u8,
        _binary_domains_build_net_shadow_end as *const u8,
    );
    type UserInit_ = fn(
        ::alloc::boxed::Box<dyn ::syscalls::Syscall>,
        ::alloc::boxed::Box<dyn ::syscalls::Heap>,
        create: alloc::sync::Arc<dyn interface::domain_create::CreateIxgbe>,
        pci: alloc::boxed::Box<dyn interface::pci::PCI>,
    ) -> alloc::boxed::Box<dyn interface::net::Net>;
    let (dom_, entry_) = unsafe { crate::domain::load_domain("net_shadow", binary_range_) };
    let user_ep_: UserInit_ = unsafe { ::core::mem::transmute::<*const (), UserInit_>(entry_) };
    let pdom_ = ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
        &dom_,
    )));
    let pheap_ = ::alloc::boxed::Box::new(crate::heap::PHeap::new());
    let thread_ = crate::thread::get_current_ref();
    let old_id_ = {
        let mut thread = thread_.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom_.lock().id;
        old_id
    };
    crate::interrupt::enable_irq();
    let ep_rtn_ = user_ep_(
        pdom_,
        pheap_,
        create: alloc::sync::Arc<dyn interface::domain_create::CreateIxgbe>,
        pci: alloc::boxed::Box<dyn interface::pci::PCI>,
    );
    crate::interrupt::disable_irq();
    {
        thread_.lock().current_domain_id = old_id_;
    }
    #[cfg(feature = "domain_create_log")]
    println!("domain/{}: returned from entry point", "net_shadow");
    let dom_: ::alloc::boxed::Box<dyn ::syscalls::Domain> = ::alloc::boxed::Box::new(
        crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom_)),
    );
    let rtn_ = (dom_, ep_rtn_);
    crate::interrupt::enable_irq();
    rtn_
}
impl interface::domain_create::CreateNvmeShadow for crate::syscalls::PDomain {
    fn create_domain_nvme_shadow(
        &self,
        create: alloc::sync::Arc<dyn interface::domain_create::CreateNvme>,
        pci: alloc::boxed::Box<dyn interface::pci::PCI>,
    ) -> (
        alloc::boxed::Box<dyn syscalls::Domain>,
        alloc::boxed::Box<dyn interface::bdev::NvmeBDev>,
    ) {
        crate::interrupt::disable_irq();
        let rtn_ = nvme_shadow_create_domain_nvme_shadow(
            create: alloc::sync::Arc<dyn interface::domain_create::CreateNvme>,
            pci: alloc::boxed::Box<dyn interface::pci::PCI>,
        );
        crate::interrupt::enable_irq();
        rtn_
    }
}
pub(crate) fn nvme_shadow_create_domain_nvme_shadow(
    create: alloc::sync::Arc<dyn interface::domain_create::CreateNvme>,
    pci: alloc::boxed::Box<dyn interface::pci::PCI>,
) -> (
    alloc::boxed::Box<dyn syscalls::Domain>,
    alloc::boxed::Box<dyn interface::bdev::NvmeBDev>,
) {
    crate::interrupt::disable_irq();
    extern "C" {
        fn _binary_domains_build_nvme_shadow_start();
        fn _binary_domains_build_nvme_shadow_end();
    }
    let binary_range_ = (
        _binary_domains_build_nvme_shadow_start as *const u8,
        _binary_domains_build_nvme_shadow_end as *const u8,
    );
    type UserInit_ = fn(
        ::alloc::boxed::Box<dyn ::syscalls::Syscall>,
        ::alloc::boxed::Box<dyn ::syscalls::Heap>,
        create: alloc::sync::Arc<dyn interface::domain_create::CreateNvme>,
        pci: alloc::boxed::Box<dyn interface::pci::PCI>,
    ) -> alloc::boxed::Box<dyn interface::bdev::NvmeBDev>;
    let (dom_, entry_) = unsafe { crate::domain::load_domain("nvme_shadow", binary_range_) };
    let user_ep_: UserInit_ = unsafe { ::core::mem::transmute::<*const (), UserInit_>(entry_) };
    let pdom_ = ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
        &dom_,
    )));
    let pheap_ = ::alloc::boxed::Box::new(crate::heap::PHeap::new());
    let thread_ = crate::thread::get_current_ref();
    let old_id_ = {
        let mut thread = thread_.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom_.lock().id;
        old_id
    };
    crate::interrupt::enable_irq();
    let ep_rtn_ = user_ep_(
        pdom_,
        pheap_,
        create: alloc::sync::Arc<dyn interface::domain_create::CreateNvme>,
        pci: alloc::boxed::Box<dyn interface::pci::PCI>,
    );
    crate::interrupt::disable_irq();
    {
        thread_.lock().current_domain_id = old_id_;
    }
    #[cfg(feature = "domain_create_log")]
    println!("domain/{}: returned from entry point", "nvme_shadow");
    let dom_: ::alloc::boxed::Box<dyn ::syscalls::Domain> = ::alloc::boxed::Box::new(
        crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom_)),
    );
    let rtn_ = (dom_, ep_rtn_);
    crate::interrupt::enable_irq();
    rtn_
}
impl interface::domain_create::CreateNvme for crate::syscalls::PDomain {
    fn create_domain_nvme(
        &self,
        pci: alloc::boxed::Box<dyn interface::pci::PCI>,
    ) -> (
        alloc::boxed::Box<dyn syscalls::Domain>,
        alloc::boxed::Box<dyn interface::bdev::NvmeBDev>,
    ) {
        crate::interrupt::disable_irq();
        let rtn_ = nvme_create_domain_nvme(pci: alloc::boxed::Box<dyn interface::pci::PCI>);
        crate::interrupt::enable_irq();
        rtn_
    }
}
pub(crate) fn nvme_create_domain_nvme(
    pci: alloc::boxed::Box<dyn interface::pci::PCI>,
) -> (
    alloc::boxed::Box<dyn syscalls::Domain>,
    alloc::boxed::Box<dyn interface::bdev::NvmeBDev>,
) {
    crate::interrupt::disable_irq();
    extern "C" {
        fn _binary_domains_build_nvme_start();
        fn _binary_domains_build_nvme_end();
    }
    let binary_range_ = (
        _binary_domains_build_nvme_start as *const u8,
        _binary_domains_build_nvme_end as *const u8,
    );
    type UserInit_ = fn(
        ::alloc::boxed::Box<dyn ::syscalls::Syscall>,
        ::alloc::boxed::Box<dyn ::syscalls::Heap>,
        pci: alloc::boxed::Box<dyn interface::pci::PCI>,
    ) -> alloc::boxed::Box<dyn interface::bdev::NvmeBDev>;
    let (dom_, entry_) = unsafe { crate::domain::load_domain("nvme", binary_range_) };
    let user_ep_: UserInit_ = unsafe { ::core::mem::transmute::<*const (), UserInit_>(entry_) };
    let pdom_ = ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
        &dom_,
    )));
    let pheap_ = ::alloc::boxed::Box::new(crate::heap::PHeap::new());
    let thread_ = crate::thread::get_current_ref();
    let old_id_ = {
        let mut thread = thread_.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom_.lock().id;
        old_id
    };
    crate::interrupt::enable_irq();
    let ep_rtn_ = user_ep_(
        pdom_,
        pheap_,
        pci: alloc::boxed::Box<dyn interface::pci::PCI>,
    );
    crate::interrupt::disable_irq();
    {
        thread_.lock().current_domain_id = old_id_;
    }
    #[cfg(feature = "domain_create_log")]
    println!("domain/{}: returned from entry point", "nvme");
    let dom_: ::alloc::boxed::Box<dyn ::syscalls::Domain> = ::alloc::boxed::Box::new(
        crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom_)),
    );
    let rtn_ = (dom_, ep_rtn_);
    crate::interrupt::enable_irq();
    rtn_
}
impl interface::domain_create::CreateRv6FS for crate::syscalls::PDomain {
    fn create_domain_xv6fs(
        &self,
        bdev: alloc::boxed::Box<dyn interface::bdev::BDev>,
    ) -> (
        alloc::boxed::Box<dyn syscalls::Domain>,
        alloc::boxed::Box<dyn interface::vfs::VFS>,
    ) {
        crate::interrupt::disable_irq();
        let rtn_ = xv6fs_create_domain_xv6fs(bdev: alloc::boxed::Box<dyn interface::bdev::BDev>);
        crate::interrupt::enable_irq();
        rtn_
    }
}
pub(crate) fn xv6fs_create_domain_xv6fs(
    bdev: alloc::boxed::Box<dyn interface::bdev::BDev>,
) -> (
    alloc::boxed::Box<dyn syscalls::Domain>,
    alloc::boxed::Box<dyn interface::vfs::VFS>,
) {
    crate::interrupt::disable_irq();
    extern "C" {
        fn _binary_domains_build_xv6fs_start();
        fn _binary_domains_build_xv6fs_end();
    }
    let binary_range_ = (
        _binary_domains_build_xv6fs_start as *const u8,
        _binary_domains_build_xv6fs_end as *const u8,
    );
    type UserInit_ = fn(
        ::alloc::boxed::Box<dyn ::syscalls::Syscall>,
        ::alloc::boxed::Box<dyn ::syscalls::Heap>,
        bdev: alloc::boxed::Box<dyn interface::bdev::BDev>,
    ) -> alloc::boxed::Box<dyn interface::vfs::VFS>;
    let (dom_, entry_) = unsafe { crate::domain::load_domain("xv6fs", binary_range_) };
    let user_ep_: UserInit_ = unsafe { ::core::mem::transmute::<*const (), UserInit_>(entry_) };
    let pdom_ = ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
        &dom_,
    )));
    let pheap_ = ::alloc::boxed::Box::new(crate::heap::PHeap::new());
    let thread_ = crate::thread::get_current_ref();
    let old_id_ = {
        let mut thread = thread_.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom_.lock().id;
        old_id
    };
    crate::interrupt::enable_irq();
    let ep_rtn_ = user_ep_(
        pdom_,
        pheap_,
        bdev: alloc::boxed::Box<dyn interface::bdev::BDev>,
    );
    crate::interrupt::disable_irq();
    {
        thread_.lock().current_domain_id = old_id_;
    }
    #[cfg(feature = "domain_create_log")]
    println!("domain/{}: returned from entry point", "xv6fs");
    let dom_: ::alloc::boxed::Box<dyn ::syscalls::Domain> = ::alloc::boxed::Box::new(
        crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom_)),
    );
    let rtn_ = (dom_, ep_rtn_);
    crate::interrupt::enable_irq();
    rtn_
}
impl interface::domain_create::CreateRv6Net for crate::syscalls::PDomain {
    fn create_domain_xv6net(
        &self,
        net: alloc::boxed::Box<dyn interface::net::Net>,
    ) -> (
        alloc::boxed::Box<dyn syscalls::Domain>,
        alloc::boxed::Box<dyn interface::usrnet::UsrNet>,
    ) {
        crate::interrupt::disable_irq();
        let rtn_ = xv6net_create_domain_xv6net(net: alloc::boxed::Box<dyn interface::net::Net>);
        crate::interrupt::enable_irq();
        rtn_
    }
}
pub(crate) fn xv6net_create_domain_xv6net(
    net: alloc::boxed::Box<dyn interface::net::Net>,
) -> (
    alloc::boxed::Box<dyn syscalls::Domain>,
    alloc::boxed::Box<dyn interface::usrnet::UsrNet>,
) {
    crate::interrupt::disable_irq();
    extern "C" {
        fn _binary_domains_build_xv6net_start();
        fn _binary_domains_build_xv6net_end();
    }
    let binary_range_ = (
        _binary_domains_build_xv6net_start as *const u8,
        _binary_domains_build_xv6net_end as *const u8,
    );
    type UserInit_ = fn(
        ::alloc::boxed::Box<dyn ::syscalls::Syscall>,
        ::alloc::boxed::Box<dyn ::syscalls::Heap>,
        net: alloc::boxed::Box<dyn interface::net::Net>,
    ) -> alloc::boxed::Box<dyn interface::usrnet::UsrNet>;
    let (dom_, entry_) = unsafe { crate::domain::load_domain("xv6net", binary_range_) };
    let user_ep_: UserInit_ = unsafe { ::core::mem::transmute::<*const (), UserInit_>(entry_) };
    let pdom_ = ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
        &dom_,
    )));
    let pheap_ = ::alloc::boxed::Box::new(crate::heap::PHeap::new());
    let thread_ = crate::thread::get_current_ref();
    let old_id_ = {
        let mut thread = thread_.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom_.lock().id;
        old_id
    };
    crate::interrupt::enable_irq();
    let ep_rtn_ = user_ep_(
        pdom_,
        pheap_,
        net: alloc::boxed::Box<dyn interface::net::Net>,
    );
    crate::interrupt::disable_irq();
    {
        thread_.lock().current_domain_id = old_id_;
    }
    #[cfg(feature = "domain_create_log")]
    println!("domain/{}: returned from entry point", "xv6net");
    let dom_: ::alloc::boxed::Box<dyn ::syscalls::Domain> = ::alloc::boxed::Box::new(
        crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom_)),
    );
    let rtn_ = (dom_, ep_rtn_);
    crate::interrupt::enable_irq();
    rtn_
}
impl interface::domain_create::CreateRv6NetShadow for crate::syscalls::PDomain {
    fn create_domain_xv6net_shadow(
        &self,
        create: alloc::sync::Arc<dyn interface::domain_create::CreateRv6Net>,
        net: alloc::boxed::Box<dyn interface::net::Net>,
    ) -> (
        alloc::boxed::Box<dyn syscalls::Domain>,
        alloc::boxed::Box<dyn interface::usrnet::UsrNet>,
    ) {
        crate::interrupt::disable_irq();
        let rtn_ = xv6net_shadow_create_domain_xv6net_shadow(
            create: alloc::sync::Arc<dyn interface::domain_create::CreateRv6Net>,
            net: alloc::boxed::Box<dyn interface::net::Net>,
        );
        crate::interrupt::enable_irq();
        rtn_
    }
}
pub(crate) fn xv6net_shadow_create_domain_xv6net_shadow(
    create: alloc::sync::Arc<dyn interface::domain_create::CreateRv6Net>,
    net: alloc::boxed::Box<dyn interface::net::Net>,
) -> (
    alloc::boxed::Box<dyn syscalls::Domain>,
    alloc::boxed::Box<dyn interface::usrnet::UsrNet>,
) {
    crate::interrupt::disable_irq();
    extern "C" {
        fn _binary_domains_build_xv6net_shadow_start();
        fn _binary_domains_build_xv6net_shadow_end();
    }
    let binary_range_ = (
        _binary_domains_build_xv6net_shadow_start as *const u8,
        _binary_domains_build_xv6net_shadow_end as *const u8,
    );
    type UserInit_ = fn(
        ::alloc::boxed::Box<dyn ::syscalls::Syscall>,
        ::alloc::boxed::Box<dyn ::syscalls::Heap>,
        create: alloc::sync::Arc<dyn interface::domain_create::CreateRv6Net>,
        net: alloc::boxed::Box<dyn interface::net::Net>,
    ) -> alloc::boxed::Box<dyn interface::usrnet::UsrNet>;
    let (dom_, entry_) = unsafe { crate::domain::load_domain("xv6net_shadow", binary_range_) };
    let user_ep_: UserInit_ = unsafe { ::core::mem::transmute::<*const (), UserInit_>(entry_) };
    let pdom_ = ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
        &dom_,
    )));
    let pheap_ = ::alloc::boxed::Box::new(crate::heap::PHeap::new());
    let thread_ = crate::thread::get_current_ref();
    let old_id_ = {
        let mut thread = thread_.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom_.lock().id;
        old_id
    };
    crate::interrupt::enable_irq();
    let ep_rtn_ = user_ep_(
        pdom_,
        pheap_,
        create: alloc::sync::Arc<dyn interface::domain_create::CreateRv6Net>,
        net: alloc::boxed::Box<dyn interface::net::Net>,
    );
    crate::interrupt::disable_irq();
    {
        thread_.lock().current_domain_id = old_id_;
    }
    #[cfg(feature = "domain_create_log")]
    println!("domain/{}: returned from entry point", "xv6net_shadow");
    let dom_: ::alloc::boxed::Box<dyn ::syscalls::Domain> = ::alloc::boxed::Box::new(
        crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom_)),
    );
    let rtn_ = (dom_, ep_rtn_);
    crate::interrupt::enable_irq();
    rtn_
}
impl interface::domain_create::CreateRv6Usr for crate::syscalls::PDomain {
    fn create_domain_xv6usr(
        &self,
        name: &str,
        blob: &[u8],
        xv6: alloc::boxed::Box<dyn interface::rv6::Rv6>,
        args: &str,
    ) -> (alloc::boxed::Box<dyn syscalls::Domain>, ()) {
        crate::interrupt::disable_irq();
        let rtn_ = xv6_user_create_domain_xv6usr(
            name: &str,
            blob: &[u8],
            xv6: alloc::boxed::Box<dyn interface::rv6::Rv6>,
            args: &str,
        );
        crate::interrupt::enable_irq();
        rtn_
    }
}
pub(crate) fn xv6_user_create_domain_xv6usr(
    name: &str,
    blob: &[u8],
    xv6: alloc::boxed::Box<dyn interface::rv6::Rv6>,
    args: &str,
) -> (alloc::boxed::Box<dyn syscalls::Domain>, ()) {
    crate::interrupt::disable_irq();
    extern "C" {
        fn _binary_domains_build_xv6_user_start();
        fn _binary_domains_build_xv6_user_end();
    }
    let begin_ = blob.as_ptr();
    let end_ = unsafe { begin_.offset(blob.len() as isize) };
    let binary_range_ = (begin_, end_);
    type UserInit_ = fn(
        ::alloc::boxed::Box<dyn ::syscalls::Syscall>,
        ::alloc::boxed::Box<dyn ::syscalls::Heap>,
        xv6: alloc::boxed::Box<dyn interface::rv6::Rv6>,
        args: &str,
    ) -> ();
    #[cfg(feature = "domain_create_log")]
    println!("Loading blob_domain/{}/{}", "xv6_user", name);
    let (dom_, entry_) = unsafe { crate::domain::load_domain("xv6_user", binary_range_) };
    let user_ep_: UserInit_ = unsafe { ::core::mem::transmute::<*const (), UserInit_>(entry_) };
    let pdom_ = ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
        &dom_,
    )));
    let pheap_ = ::alloc::boxed::Box::new(crate::heap::PHeap::new());
    let thread_ = crate::thread::get_current_ref();
    let old_id_ = {
        let mut thread = thread_.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom_.lock().id;
        old_id
    };
    crate::interrupt::enable_irq();
    let ep_rtn_ = user_ep_(
        pdom_,
        pheap_,
        xv6: alloc::boxed::Box<dyn interface::rv6::Rv6>,
        args: &str,
    );
    crate::interrupt::disable_irq();
    {
        thread_.lock().current_domain_id = old_id_;
    }
    #[cfg(feature = "domain_create_log")]
    println!(
        "blob_domain/{}/{}: returned from entry point",
        "xv6_user", name
    );
    let dom_: ::alloc::boxed::Box<dyn ::syscalls::Domain> = ::alloc::boxed::Box::new(
        crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom_)),
    );
    let rtn_ = (dom_, ep_rtn_);
    crate::interrupt::enable_irq();
    rtn_
}
impl interface::domain_create::CreateRv6 for crate::syscalls::PDomain {
    fn create_domain_xv6kernel(
        &self,
        ints: alloc::boxed::Box<dyn syscalls::Interrupt>,
        create_xv6fs: alloc::sync::Arc<dyn interface::domain_create::CreateRv6FS>,
        create_xv6net: alloc::sync::Arc<dyn interface::domain_create::CreateRv6Net>,
        create_xv6net_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateRv6NetShadow>,
        create_xv6usr: alloc::sync::Arc<dyn interface::domain_create::CreateRv6Usr>,
        bdev: alloc::boxed::Box<dyn interface::bdev::BDev>,
        net: alloc::boxed::Box<dyn interface::net::Net>,
        nvme: alloc::boxed::Box<dyn interface::bdev::NvmeBDev>,
        usr_tpm: alloc::boxed::Box<dyn interface::tpm::UsrTpm>,
    ) -> (
        alloc::boxed::Box<dyn syscalls::Domain>,
        alloc::boxed::Box<dyn interface::rv6::Rv6>,
    ) {
        crate::interrupt::disable_irq();
        let rtn_ = xv6kernel_create_domain_xv6kernel(
            ints: alloc::boxed::Box<dyn syscalls::Interrupt>,
            create_xv6fs: alloc::sync::Arc<dyn interface::domain_create::CreateRv6FS>,
            create_xv6net: alloc::sync::Arc<dyn interface::domain_create::CreateRv6Net>,
            create_xv6net_shadow:
                alloc::sync::Arc<dyn interface::domain_create::CreateRv6NetShadow>,
            create_xv6usr: alloc::sync::Arc<dyn interface::domain_create::CreateRv6Usr>,
            bdev: alloc::boxed::Box<dyn interface::bdev::BDev>,
            net: alloc::boxed::Box<dyn interface::net::Net>,
            nvme: alloc::boxed::Box<dyn interface::bdev::NvmeBDev>,
            usr_tpm: alloc::boxed::Box<dyn interface::tpm::UsrTpm>,
        );
        crate::interrupt::enable_irq();
        rtn_
    }
}
pub(crate) fn xv6kernel_create_domain_xv6kernel(
    ints: alloc::boxed::Box<dyn syscalls::Interrupt>,
    create_xv6fs: alloc::sync::Arc<dyn interface::domain_create::CreateRv6FS>,
    create_xv6net: alloc::sync::Arc<dyn interface::domain_create::CreateRv6Net>,
    create_xv6net_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateRv6NetShadow>,
    create_xv6usr: alloc::sync::Arc<dyn interface::domain_create::CreateRv6Usr>,
    bdev: alloc::boxed::Box<dyn interface::bdev::BDev>,
    net: alloc::boxed::Box<dyn interface::net::Net>,
    nvme: alloc::boxed::Box<dyn interface::bdev::NvmeBDev>,
    usr_tpm: alloc::boxed::Box<dyn interface::tpm::UsrTpm>,
) -> (
    alloc::boxed::Box<dyn syscalls::Domain>,
    alloc::boxed::Box<dyn interface::rv6::Rv6>,
) {
    crate::interrupt::disable_irq();
    extern "C" {
        fn _binary_domains_build_xv6kernel_start();
        fn _binary_domains_build_xv6kernel_end();
    }
    let binary_range_ = (
        _binary_domains_build_xv6kernel_start as *const u8,
        _binary_domains_build_xv6kernel_end as *const u8,
    );
    type UserInit_ = fn(
        ::alloc::boxed::Box<dyn ::syscalls::Syscall>,
        ::alloc::boxed::Box<dyn ::syscalls::Heap>,
        ints: alloc::boxed::Box<dyn syscalls::Interrupt>,
        create_xv6fs: alloc::sync::Arc<dyn interface::domain_create::CreateRv6FS>,
        create_xv6net: alloc::sync::Arc<dyn interface::domain_create::CreateRv6Net>,
        create_xv6net_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateRv6NetShadow>,
        create_xv6usr: alloc::sync::Arc<dyn interface::domain_create::CreateRv6Usr>,
        bdev: alloc::boxed::Box<dyn interface::bdev::BDev>,
        net: alloc::boxed::Box<dyn interface::net::Net>,
        nvme: alloc::boxed::Box<dyn interface::bdev::NvmeBDev>,
        usr_tpm: alloc::boxed::Box<dyn interface::tpm::UsrTpm>,
    ) -> alloc::boxed::Box<dyn interface::rv6::Rv6>;
    let (dom_, entry_) = unsafe { crate::domain::load_domain("xv6kernel", binary_range_) };
    let user_ep_: UserInit_ = unsafe { ::core::mem::transmute::<*const (), UserInit_>(entry_) };
    let pdom_ = ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
        &dom_,
    )));
    let pheap_ = ::alloc::boxed::Box::new(crate::heap::PHeap::new());
    let thread_ = crate::thread::get_current_ref();
    let old_id_ = {
        let mut thread = thread_.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom_.lock().id;
        old_id
    };
    crate::interrupt::enable_irq();
    let ep_rtn_ = user_ep_(
        pdom_,
        pheap_,
        ints: alloc::boxed::Box<dyn syscalls::Interrupt>,
        create_xv6fs: alloc::sync::Arc<dyn interface::domain_create::CreateRv6FS>,
        create_xv6net: alloc::sync::Arc<dyn interface::domain_create::CreateRv6Net>,
        create_xv6net_shadow: alloc::sync::Arc<dyn interface::domain_create::CreateRv6NetShadow>,
        create_xv6usr: alloc::sync::Arc<dyn interface::domain_create::CreateRv6Usr>,
        bdev: alloc::boxed::Box<dyn interface::bdev::BDev>,
        net: alloc::boxed::Box<dyn interface::net::Net>,
        nvme: alloc::boxed::Box<dyn interface::bdev::NvmeBDev>,
        usr_tpm: alloc::boxed::Box<dyn interface::tpm::UsrTpm>,
    );
    crate::interrupt::disable_irq();
    {
        thread_.lock().current_domain_id = old_id_;
    }
    #[cfg(feature = "domain_create_log")]
    println!("domain/{}: returned from entry point", "xv6kernel");
    let dom_: ::alloc::boxed::Box<dyn ::syscalls::Domain> = ::alloc::boxed::Box::new(
        crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom_)),
    );
    let rtn_ = (dom_, ep_rtn_);
    crate::interrupt::enable_irq();
    rtn_
}
impl interface::domain_create::CreateDomC for crate::syscalls::PDomain {
    fn create_domain_dom_c(
        &self,
    ) -> (
        alloc::boxed::Box<dyn syscalls::Domain>,
        alloc::boxed::Box<dyn interface::dom_c::DomC>,
    ) {
        crate::interrupt::disable_irq();
        let rtn_ = dom_c_create_domain_dom_c();
        crate::interrupt::enable_irq();
        rtn_
    }
    fn recreate_domain_dom_c(
        &self,
        dom: alloc::boxed::Box<dyn syscalls::Domain>,
    ) -> (
        alloc::boxed::Box<dyn syscalls::Domain>,
        alloc::boxed::Box<dyn interface::dom_c::DomC>,
    ) {
        crate::interrupt::disable_irq();
        let rtn_ = dom_c_recreate_domain_dom_c(dom: alloc::boxed::Box<dyn syscalls::Domain>);
        crate::interrupt::enable_irq();
        rtn_
    }
}
pub(crate) fn dom_c_create_domain_dom_c() -> (
    alloc::boxed::Box<dyn syscalls::Domain>,
    alloc::boxed::Box<dyn interface::dom_c::DomC>,
) {
    crate::interrupt::disable_irq();
    extern "C" {
        fn _binary_domains_build_dom_c_start();
        fn _binary_domains_build_dom_c_end();
    }
    let binary_range_ = (
        _binary_domains_build_dom_c_start as *const u8,
        _binary_domains_build_dom_c_end as *const u8,
    );
    type UserInit_ = fn(
        ::alloc::boxed::Box<dyn ::syscalls::Syscall>,
        ::alloc::boxed::Box<dyn ::syscalls::Heap>,
    ) -> alloc::boxed::Box<dyn interface::dom_c::DomC>;
    let (dom_, entry_) = unsafe { crate::domain::load_domain("dom_c", binary_range_) };
    let user_ep_: UserInit_ = unsafe { ::core::mem::transmute::<*const (), UserInit_>(entry_) };
    let pdom_ = ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
        &dom_,
    )));
    let pheap_ = ::alloc::boxed::Box::new(crate::heap::PHeap::new());
    let thread_ = crate::thread::get_current_ref();
    let old_id_ = {
        let mut thread = thread_.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom_.lock().id;
        old_id
    };
    crate::interrupt::enable_irq();
    let ep_rtn_ = user_ep_(pdom_, pheap_);
    crate::interrupt::disable_irq();
    {
        thread_.lock().current_domain_id = old_id_;
    }
    #[cfg(feature = "domain_create_log")]
    println!("domain/{}: returned from entry point", "dom_c");
    let dom_: ::alloc::boxed::Box<dyn ::syscalls::Domain> = ::alloc::boxed::Box::new(
        crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom_)),
    );
    let rtn_ = (dom_, ep_rtn_);
    crate::interrupt::enable_irq();
    rtn_
}
pub(crate) fn dom_c_recreate_domain_dom_c(
    dom: alloc::boxed::Box<dyn syscalls::Domain>,
) -> (
    alloc::boxed::Box<dyn syscalls::Domain>,
    alloc::boxed::Box<dyn interface::dom_c::DomC>,
) {
    crate::interrupt::disable_irq();
    extern "C" {
        fn _binary_domains_build_dom_c_start();
        fn _binary_domains_build_dom_c_end();
    }
    let binary_range_ = (
        _binary_domains_build_dom_c_start as *const u8,
        _binary_domains_build_dom_c_end as *const u8,
    );
    type UserInit_ = fn(
        ::alloc::boxed::Box<dyn ::syscalls::Syscall>,
        ::alloc::boxed::Box<dyn ::syscalls::Heap>,
        dom: alloc::boxed::Box<dyn syscalls::Domain>,
    ) -> alloc::boxed::Box<dyn interface::dom_c::DomC>;
    let (dom_, entry_) = unsafe { crate::domain::load_domain("dom_c", binary_range_) };
    let user_ep_: UserInit_ = unsafe { ::core::mem::transmute::<*const (), UserInit_>(entry_) };
    let pdom_ = ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
        &dom_,
    )));
    let pheap_ = ::alloc::boxed::Box::new(crate::heap::PHeap::new());
    let thread_ = crate::thread::get_current_ref();
    let old_id_ = {
        let mut thread = thread_.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom_.lock().id;
        old_id
    };
    crate::interrupt::enable_irq();
    let ep_rtn_ = user_ep_(pdom_, pheap_, dom: alloc::boxed::Box<dyn syscalls::Domain>);
    crate::interrupt::disable_irq();
    {
        thread_.lock().current_domain_id = old_id_;
    }
    #[cfg(feature = "domain_create_log")]
    println!("domain/{}: returned from entry point", "dom_c");
    let dom_: ::alloc::boxed::Box<dyn ::syscalls::Domain> = ::alloc::boxed::Box::new(
        crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom_)),
    );
    let rtn_ = (dom_, ep_rtn_);
    crate::interrupt::enable_irq();
    rtn_
}
impl interface::domain_create::CreateDomD for crate::syscalls::PDomain {
    fn create_domain_dom_d(
        &self,
        dom_c: alloc::boxed::Box<dyn interface::dom_c::DomC>,
    ) -> (alloc::boxed::Box<dyn syscalls::Domain>, ()) {
        crate::interrupt::disable_irq();
        let rtn_ = dom_d_create_domain_dom_d(dom_c: alloc::boxed::Box<dyn interface::dom_c::DomC>);
        crate::interrupt::enable_irq();
        rtn_
    }
}
pub(crate) fn dom_d_create_domain_dom_d(
    dom_c: alloc::boxed::Box<dyn interface::dom_c::DomC>,
) -> (alloc::boxed::Box<dyn syscalls::Domain>, ()) {
    crate::interrupt::disable_irq();
    extern "C" {
        fn _binary_domains_build_dom_d_start();
        fn _binary_domains_build_dom_d_end();
    }
    let binary_range_ = (
        _binary_domains_build_dom_d_start as *const u8,
        _binary_domains_build_dom_d_end as *const u8,
    );
    type UserInit_ = fn(
        ::alloc::boxed::Box<dyn ::syscalls::Syscall>,
        ::alloc::boxed::Box<dyn ::syscalls::Heap>,
        dom_c: alloc::boxed::Box<dyn interface::dom_c::DomC>,
    ) -> ();
    let (dom_, entry_) = unsafe { crate::domain::load_domain("dom_d", binary_range_) };
    let user_ep_: UserInit_ = unsafe { ::core::mem::transmute::<*const (), UserInit_>(entry_) };
    let pdom_ = ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
        &dom_,
    )));
    let pheap_ = ::alloc::boxed::Box::new(crate::heap::PHeap::new());
    let thread_ = crate::thread::get_current_ref();
    let old_id_ = {
        let mut thread = thread_.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom_.lock().id;
        old_id
    };
    crate::interrupt::enable_irq();
    let ep_rtn_ = user_ep_(
        pdom_,
        pheap_,
        dom_c: alloc::boxed::Box<dyn interface::dom_c::DomC>,
    );
    crate::interrupt::disable_irq();
    {
        thread_.lock().current_domain_id = old_id_;
    }
    #[cfg(feature = "domain_create_log")]
    println!("domain/{}: returned from entry point", "dom_d");
    let dom_: ::alloc::boxed::Box<dyn ::syscalls::Domain> = ::alloc::boxed::Box::new(
        crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom_)),
    );
    let rtn_ = (dom_, ep_rtn_);
    crate::interrupt::enable_irq();
    rtn_
}
impl interface::domain_create::CreateShadow for crate::syscalls::PDomain {
    fn create_domain_shadow(
        &self,
        create_dom_c: alloc::sync::Arc<dyn interface::domain_create::CreateDomC>,
    ) -> (
        alloc::boxed::Box<dyn syscalls::Domain>,
        alloc::boxed::Box<dyn interface::dom_c::DomC>,
    ) {
        crate::interrupt::disable_irq();
        let rtn_ = shadow_create_domain_shadow(
            create_dom_c: alloc::sync::Arc<dyn interface::domain_create::CreateDomC>,
        );
        crate::interrupt::enable_irq();
        rtn_
    }
}
pub(crate) fn shadow_create_domain_shadow(
    create_dom_c: alloc::sync::Arc<dyn interface::domain_create::CreateDomC>,
) -> (
    alloc::boxed::Box<dyn syscalls::Domain>,
    alloc::boxed::Box<dyn interface::dom_c::DomC>,
) {
    crate::interrupt::disable_irq();
    extern "C" {
        fn _binary_domains_build_shadow_start();
        fn _binary_domains_build_shadow_end();
    }
    let binary_range_ = (
        _binary_domains_build_shadow_start as *const u8,
        _binary_domains_build_shadow_end as *const u8,
    );
    type UserInit_ = fn(
        ::alloc::boxed::Box<dyn ::syscalls::Syscall>,
        ::alloc::boxed::Box<dyn ::syscalls::Heap>,
        create_dom_c: alloc::sync::Arc<dyn interface::domain_create::CreateDomC>,
    ) -> alloc::boxed::Box<dyn interface::dom_c::DomC>;
    let (dom_, entry_) = unsafe { crate::domain::load_domain("shadow", binary_range_) };
    let user_ep_: UserInit_ = unsafe { ::core::mem::transmute::<*const (), UserInit_>(entry_) };
    let pdom_ = ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
        &dom_,
    )));
    let pheap_ = ::alloc::boxed::Box::new(crate::heap::PHeap::new());
    let thread_ = crate::thread::get_current_ref();
    let old_id_ = {
        let mut thread = thread_.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom_.lock().id;
        old_id
    };
    crate::interrupt::enable_irq();
    let ep_rtn_ = user_ep_(
        pdom_,
        pheap_,
        create_dom_c: alloc::sync::Arc<dyn interface::domain_create::CreateDomC>,
    );
    crate::interrupt::disable_irq();
    {
        thread_.lock().current_domain_id = old_id_;
    }
    #[cfg(feature = "domain_create_log")]
    println!("domain/{}: returned from entry point", "shadow");
    let dom_: ::alloc::boxed::Box<dyn ::syscalls::Domain> = ::alloc::boxed::Box::new(
        crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom_)),
    );
    let rtn_ = (dom_, ep_rtn_);
    crate::interrupt::enable_irq();
    rtn_
}
impl interface::domain_create::CreateBenchnvme for crate::syscalls::PDomain {
    fn create_domain_benchnvme(
        &self,
        nvme: alloc::boxed::Box<dyn interface::bdev::NvmeBDev>,
    ) -> (alloc::boxed::Box<dyn syscalls::Domain>, ()) {
        crate::interrupt::disable_irq();
        let rtn_ = benchnvme_create_domain_benchnvme(
            nvme: alloc::boxed::Box<dyn interface::bdev::NvmeBDev>,
        );
        crate::interrupt::enable_irq();
        rtn_
    }
}
pub(crate) fn benchnvme_create_domain_benchnvme(
    nvme: alloc::boxed::Box<dyn interface::bdev::NvmeBDev>,
) -> (alloc::boxed::Box<dyn syscalls::Domain>, ()) {
    crate::interrupt::disable_irq();
    extern "C" {
        fn _binary_domains_build_benchnvme_start();
        fn _binary_domains_build_benchnvme_end();
    }
    let binary_range_ = (
        _binary_domains_build_benchnvme_start as *const u8,
        _binary_domains_build_benchnvme_end as *const u8,
    );
    type UserInit_ = fn(
        ::alloc::boxed::Box<dyn ::syscalls::Syscall>,
        ::alloc::boxed::Box<dyn ::syscalls::Heap>,
        nvme: alloc::boxed::Box<dyn interface::bdev::NvmeBDev>,
    ) -> ();
    let (dom_, entry_) = unsafe { crate::domain::load_domain("benchnvme", binary_range_) };
    let user_ep_: UserInit_ = unsafe { ::core::mem::transmute::<*const (), UserInit_>(entry_) };
    let pdom_ = ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
        &dom_,
    )));
    let pheap_ = ::alloc::boxed::Box::new(crate::heap::PHeap::new());
    let thread_ = crate::thread::get_current_ref();
    let old_id_ = {
        let mut thread = thread_.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom_.lock().id;
        old_id
    };
    crate::interrupt::enable_irq();
    let ep_rtn_ = user_ep_(
        pdom_,
        pheap_,
        nvme: alloc::boxed::Box<dyn interface::bdev::NvmeBDev>,
    );
    crate::interrupt::disable_irq();
    {
        thread_.lock().current_domain_id = old_id_;
    }
    #[cfg(feature = "domain_create_log")]
    println!("domain/{}: returned from entry point", "benchnvme");
    let dom_: ::alloc::boxed::Box<dyn ::syscalls::Domain> = ::alloc::boxed::Box::new(
        crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom_)),
    );
    let rtn_ = (dom_, ep_rtn_);
    crate::interrupt::enable_irq();
    rtn_
}
impl interface::domain_create::CreateTpm for crate::syscalls::PDomain {
    fn create_domain_tpm(
        &self,
    ) -> (
        alloc::boxed::Box<dyn syscalls::Domain>,
        alloc::boxed::Box<dyn interface::tpm::UsrTpm>,
    ) {
        crate::interrupt::disable_irq();
        let rtn_ = tpm_create_domain_tpm();
        crate::interrupt::enable_irq();
        rtn_
    }
}
pub(crate) fn tpm_create_domain_tpm() -> (
    alloc::boxed::Box<dyn syscalls::Domain>,
    alloc::boxed::Box<dyn interface::tpm::UsrTpm>,
) {
    crate::interrupt::disable_irq();
    extern "C" {
        fn _binary_domains_build_tpm_start();
        fn _binary_domains_build_tpm_end();
    }
    let binary_range_ = (
        _binary_domains_build_tpm_start as *const u8,
        _binary_domains_build_tpm_end as *const u8,
    );
    type UserInit_ = fn(
        ::alloc::boxed::Box<dyn ::syscalls::Syscall>,
        ::alloc::boxed::Box<dyn ::syscalls::Heap>,
    ) -> alloc::boxed::Box<dyn interface::tpm::UsrTpm>;
    let (dom_, entry_) = unsafe { crate::domain::load_domain("tpm", binary_range_) };
    let user_ep_: UserInit_ = unsafe { ::core::mem::transmute::<*const (), UserInit_>(entry_) };
    let pdom_ = ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
        &dom_,
    )));
    let pheap_ = ::alloc::boxed::Box::new(crate::heap::PHeap::new());
    let thread_ = crate::thread::get_current_ref();
    let old_id_ = {
        let mut thread = thread_.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom_.lock().id;
        old_id
    };
    crate::interrupt::enable_irq();
    let ep_rtn_ = user_ep_(pdom_, pheap_);
    crate::interrupt::disable_irq();
    {
        thread_.lock().current_domain_id = old_id_;
    }
    #[cfg(feature = "domain_create_log")]
    println!("domain/{}: returned from entry point", "tpm");
    let dom_: ::alloc::boxed::Box<dyn ::syscalls::Domain> = ::alloc::boxed::Box::new(
        crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(&dom_)),
    );
    let rtn_ = (dom_, ep_rtn_);
    crate::interrupt::enable_irq();
    rtn_
}
pub fn create_domain_init() -> ::alloc::boxed::Box<dyn ::syscalls::Domain> {
    let name = "init";
    extern "C" {
        fn _binary_domains_build_redleaf_init_start();
        fn _binary_domains_build_redleaf_init_end();
    }
    let binary_range = (
        _binary_domains_build_redleaf_init_start as *const u8,
        _binary_domains_build_redleaf_init_end as *const u8,
    );
    type UserInit = fn(
        ::alloc::boxed::Box<dyn ::syscalls::Syscall + Send + Sync>,
        ::alloc::boxed::Box<dyn ::syscalls::Heap + Send + Sync>,
        ::alloc::boxed::Box<dyn ::syscalls::Interrupt>,
        ::alloc::sync::Arc<dyn interface::domain_create::CreateProxy>,
        ::alloc::sync::Arc<dyn interface::domain_create::CreatePCI>,
        ::alloc::sync::Arc<dyn interface::domain_create::CreateMemBDev>,
        ::alloc::sync::Arc<dyn interface::domain_create::CreateBDevShadow>,
        ::alloc::sync::Arc<dyn interface::domain_create::CreateIxgbe>,
        ::alloc::sync::Arc<dyn interface::domain_create::CreateVirtioNet>,
        ::alloc::sync::Arc<dyn interface::domain_create::CreateVirtioBlock>,
        ::alloc::sync::Arc<dyn interface::domain_create::CreateNetShadow>,
        ::alloc::sync::Arc<dyn interface::domain_create::CreateNvmeShadow>,
        ::alloc::sync::Arc<dyn interface::domain_create::CreateNvme>,
        ::alloc::sync::Arc<dyn interface::domain_create::CreateRv6FS>,
        ::alloc::sync::Arc<dyn interface::domain_create::CreateRv6Net>,
        ::alloc::sync::Arc<dyn interface::domain_create::CreateRv6NetShadow>,
        ::alloc::sync::Arc<dyn interface::domain_create::CreateRv6Usr>,
        ::alloc::sync::Arc<dyn interface::domain_create::CreateRv6>,
        ::alloc::sync::Arc<dyn interface::domain_create::CreateDomC>,
        ::alloc::sync::Arc<dyn interface::domain_create::CreateDomD>,
        ::alloc::sync::Arc<dyn interface::domain_create::CreateShadow>,
        ::alloc::sync::Arc<dyn interface::domain_create::CreateBenchnvme>,
        ::alloc::sync::Arc<dyn interface::domain_create::CreateTpm>,
    );
    let (dom, entry) = unsafe { crate::domain::load_domain(name, binary_range) };
    let user_ep: UserInit = unsafe { ::core::mem::transmute::<*const (), UserInit>(entry) };
    let thread = crate::thread::get_current_ref();
    let old_id = {
        let mut thread = thread.lock();
        let old_id = thread.current_domain_id;
        thread.current_domain_id = dom.lock().id;
        old_id
    };
    crate::interrupt::enable_irq();
    user_ep(
        ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
            &dom,
        ))),
        ::alloc::boxed::Box::new(crate::heap::PHeap::new()),
        ::alloc::boxed::Box::new(crate::syscalls::Interrupt::new()),
        ::alloc::sync::Arc::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
            &dom,
        ))),
        ::alloc::sync::Arc::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
            &dom,
        ))),
        ::alloc::sync::Arc::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
            &dom,
        ))),
        ::alloc::sync::Arc::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
            &dom,
        ))),
        ::alloc::sync::Arc::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
            &dom,
        ))),
        ::alloc::sync::Arc::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
            &dom,
        ))),
        ::alloc::sync::Arc::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
            &dom,
        ))),
        ::alloc::sync::Arc::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
            &dom,
        ))),
        ::alloc::sync::Arc::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
            &dom,
        ))),
        ::alloc::sync::Arc::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
            &dom,
        ))),
        ::alloc::sync::Arc::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
            &dom,
        ))),
        ::alloc::sync::Arc::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
            &dom,
        ))),
        ::alloc::sync::Arc::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
            &dom,
        ))),
        ::alloc::sync::Arc::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
            &dom,
        ))),
        ::alloc::sync::Arc::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
            &dom,
        ))),
        ::alloc::sync::Arc::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
            &dom,
        ))),
        ::alloc::sync::Arc::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
            &dom,
        ))),
        ::alloc::sync::Arc::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
            &dom,
        ))),
        ::alloc::sync::Arc::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
            &dom,
        ))),
        ::alloc::sync::Arc::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
            &dom,
        ))),
    );
    crate::interrupt::disable_irq();
    {
        thread.lock().current_domain_id = old_id;
    }
    #[cfg(feature = "domain_create_log")]
    println!("domain/{}: returned from entry point", name);
    ::alloc::boxed::Box::new(crate::syscalls::PDomain::new(::alloc::sync::Arc::clone(
        &dom,
    )))
}
