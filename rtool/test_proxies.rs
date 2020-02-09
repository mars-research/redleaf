pub fn sys_bdev_read_new_data(&self, data: [u8; 512]) -> RRef<[u8; 512]> {
	let proxy = PROXY.r#try().expect("Proxy interface is not initialized.");
	proxy.bdev_read_new_data(data);
}

pub fn sys_bdev_read_drop_data(&self, data: RRef<[u8; 512]>) {
	let proxy = PROXY.r#try().expect("Proxy interface is not initialized.");
	proxy.bdev_read_drop_data(data);
}

pub fn sys_read(block: u32, data: &mut RRef<[u8; 512]>) -> RRef<[u8; 512]> {
	let proxy = PROXY.r#try().expect("Proxy interface is not initialized.");
	proxy.bdev_read(block, data);
}

pub fn sys_bdev_write_new_data(&self, data: [u8; 512]) -> RRef<[u8; 512]> {
	let proxy = PROXY.r#try().expect("Proxy interface is not initialized.");
	proxy.bdev_write_new_data(data);
}

pub fn sys_bdev_write_drop_data(&self, data: RRef<[u8; 512]>) {
	let proxy = PROXY.r#try().expect("Proxy interface is not initialized.");
	proxy.bdev_write_drop_data(data);
}

pub fn sys_write(block: u32, data: &RRef<[u8; 512]>) {
	let proxy = PROXY.r#try().expect("Proxy interface is not initialized.");
	proxy.bdev_write(block, data);
}

pub trait Proxy {
fn proxy_clone(&self) -> Box<dyn Proxy>;
	fn bdev_read_new_data(&self, data: [u8; 512]) -> RRef<[u8; 512]>;
	fn bdev_read_drop_data(&self, data: RRef<[u8; 512]>);
	fn bdev_read(&self, block: u32, data: &mut RRef<[u8; 512]>) -> RRef<[u8; 512]>;
	fn bdev_write_new_data(&self, data: [u8; 512]) -> RRef<[u8; 512]>;
	fn bdev_write_drop_data(&self, data: RRef<[u8; 512]>);
	fn bdev_write(&self, block: u32, data: &RRef<[u8; 512]>);
}

impl usr::proxy::Proxy for Proxy {
	fn proxy_clone(&self) -> Box<dyn usr::proxy::Proxy> {
		Box::new((*self).clone());
	}

	fn bdev_read_new_data(&self, data: [u8; 512]) -> RRef<[u8; 512]> {
		let rref = RRef::new(0, data);
		rref;
	}

	fn bdev_read_drop_data(&self, data: RRef<[u8; 512]>) {
		RRef::drop(data);
	}

	fn bdev_read(&self, block: u32, data: &mut RRef<[u8; 512]>) -> RRef<[u8; 512]> {
		let bdev = self.bdev.as_deref().expect("BDev interface not initialized.");
		bdev.read(block, data);
	}

	fn bdev_write_new_data(&self, data: [u8; 512]) -> RRef<[u8; 512]> {
		let rref = RRef::new(0, data);
		rref;
	}

	fn bdev_write_drop_data(&self, data: RRef<[u8; 512]>) {
		RRef::drop(data);
	}

	fn bdev_write(&self, block: u32, data: &RRef<[u8; 512]>) {
		let bdev = self.bdev.as_deref().expect("BDev interface not initialized.");
		bdev.write(block, data);
	}

}
