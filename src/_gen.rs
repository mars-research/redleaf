use syscalls;
use create;
use create::*;
use proxy;
use usr;
use spin::Mutex;
use alloc::sync::Arc;
use alloc::boxed::Box;
use usr::error::Result;
use crate::domain::load_domain;
use crate::syscalls::{PDomain, Interrupt, Mmap};
use crate::heap::PHeap;
use crate::interrupt::{disable_irq, enable_irq};
use crate::thread;

impl create::CreatePCI for PDomain {
	fn create_domain_pci ( & self ) -> ( Box < dyn syscalls :: Domain > , Box < dyn usr :: pci :: PCI > ) {
		disable_irq();
		let r = create_domain_pci();
		enable_irq();
		r
	}
}

impl create::CreateAHCI for PDomain {
	fn create_domain_ahci ( & self , pci : Box < dyn usr :: pci :: PCI > ) -> ( Box < dyn syscalls :: Domain > , Box < dyn usr :: bdev :: BDev > ) {
		disable_irq();
		let r = create_domain_ahci(pci);
		enable_irq();
		r
	}
}

impl create::CreateMemBDev for PDomain {
	fn create_domain_membdev ( & self , memdisk : & 'static mut [ u8 ] ) -> ( Box < dyn syscalls :: Domain > , Box < dyn usr :: bdev :: BDev > ) {
		disable_irq();
		let r = create_domain_membdev(memdisk);
		enable_irq();
		r
	}
	fn recreate_domain_membdev ( & self , dom : Box < dyn syscalls :: Domain > , memdisk : & 'static mut [ u8 ] ) -> ( Box < dyn syscalls :: Domain > , Box < dyn usr :: bdev :: BDev > ) {
		disable_irq();
		let r = recreate_domain_membdev(dom, memdisk);
		enable_irq();
		r
	}
}

impl create::CreateBDevShadow for PDomain {
	fn create_domain_bdev_shadow ( & self , create : Arc < dyn CreateMemBDev > ) -> ( Box < dyn syscalls :: Domain > , Box < dyn usr :: bdev :: BDev > ) {
		disable_irq();
		let r = create_domain_bdev_shadow(create);
		enable_irq();
		r
	}
}

impl create::CreateIxgbe for PDomain {
	fn create_domain_ixgbe ( & self , pci : Box < dyn usr :: pci :: PCI > ) -> ( Box < dyn syscalls :: Domain > , Box < dyn usr :: net :: Net + Send > ) {
		disable_irq();
		let r = create_domain_ixgbe(pci);
		enable_irq();
		r
	}
}

impl create::CreateNetShadow for PDomain {
	fn create_domain_net_shadow ( & self , create : Arc < dyn CreateIxgbe > , pci : Box < dyn usr :: pci :: PCI > ) -> ( Box < dyn syscalls :: Domain > , Box < dyn usr :: net :: Net + Send > ) {
		disable_irq();
		let r = create_domain_net_shadow(create, pci);
		enable_irq();
		r
	}
}

impl create::CreateNvme for PDomain {
	fn create_domain_nvme ( & self , pci : Box < dyn usr :: pci :: PCI > ) -> Box < dyn syscalls :: Domain > {
		disable_irq();
		let r = create_domain_nvme(pci);
		enable_irq();
		r
	}
}

impl create::CreateXv6FS for PDomain {
	fn create_domain_xv6fs ( & self , bdev : Box < dyn usr :: bdev :: BDev > ) -> ( Box < dyn syscalls :: Domain > , Box < dyn usr :: vfs :: VFS + Send > ) {
		disable_irq();
		let r = create_domain_xv6fs(bdev);
		enable_irq();
		r
	}
}

impl create::CreateXv6Usr for PDomain {
	fn create_domain_xv6usr ( & self , name : & str , xv6 : Box < dyn usr :: xv6 :: Xv6 > , blob : & [ u8 ] , args : & str ) -> Result < Box < dyn syscalls :: Domain > > {
		disable_irq();
		let r = create_domain_xv6usr(name, xv6, blob, args);
		enable_irq();
		r
	}
}

impl create::CreateXv6 for PDomain {
	fn create_domain_xv6kernel ( & self , ints : Box < dyn Interrupt > , create_xv6fs : Arc < dyn CreateXv6FS > , create_xv6usr : Arc < dyn CreateXv6Usr + Send + Sync > , bdev : Box < dyn usr :: bdev :: BDev > , net : Box < dyn usr :: net :: Net > ) -> ( Box < dyn syscalls :: Domain > , Box < dyn usr :: xv6 :: Xv6 > ) {
		disable_irq();
		let r = create_domain_xv6kernel(ints, create_xv6fs, create_xv6usr, bdev, net);
		enable_irq();
		r
	}
}

impl create::CreateDomA for PDomain {
	fn create_domain_dom_a ( & self ) -> ( Box < dyn syscalls :: Domain > , Box < dyn usr :: dom_a :: DomA > ) {
		disable_irq();
		let r = create_domain_dom_a();
		enable_irq();
		r
	}
}

impl create::CreateDomB for PDomain {
	fn create_domain_dom_b ( & self , dom_a : Box < dyn usr :: dom_a :: DomA > ) -> Box < dyn syscalls :: Domain > {
		disable_irq();
		let r = create_domain_dom_b(dom_a);
		enable_irq();
		r
	}
}

impl create::CreateDomC for PDomain {
	fn create_domain_dom_c ( & self ) -> ( Box < dyn syscalls :: Domain > , Box < dyn usr :: dom_c :: DomC > ) {
		disable_irq();
		let r = create_domain_dom_c();
		enable_irq();
		r
	}
	fn recreate_domain_dom_c ( & self , dom : Box < dyn syscalls :: Domain > ) -> ( Box < dyn syscalls :: Domain > , Box < dyn usr :: dom_c :: DomC > ) {
		disable_irq();
		let r = recreate_domain_dom_c(dom);
		enable_irq();
		r
	}
}

impl create::CreateDomD for PDomain {
	fn create_domain_dom_d ( & self , dom_c : Box < dyn usr :: dom_c :: DomC > ) -> Box < dyn syscalls :: Domain > {
		disable_irq();
		let r = create_domain_dom_d(dom_c);
		enable_irq();
		r
	}
}

impl create::CreateShadow for PDomain {
	fn create_domain_shadow ( & self , create_dom_c : Arc < dyn CreateDomC > ) -> ( Box < dyn syscalls :: Domain > , Box < dyn usr :: dom_c :: DomC > ) {
		disable_irq();
		let r = create_domain_shadow(create_dom_c);
		enable_irq();
		r
	}
}

impl create::CreateBenchnet for PDomain {
	fn create_domain_benchnet ( & self , net : Box < dyn usr :: net :: Net > ) -> Box < dyn syscalls :: Domain > {
		disable_irq();
		let r = create_domain_benchnet(net);
		enable_irq();
		r
	}
}

