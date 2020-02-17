pub trait BDev {
    fn read(&self, block: u32, data: &mut RRef<[u8; 512]>) -> RRef<[u8; 512]>;
    fn write(&self, block: u32, data: &RRef<[u8; 512]>);
}

pub trait BDev2 {
    fn read(&self, block: u32, data: &mut RRef<[u8; 512]>) -> RRef<[u8; 512]>;
    fn write(&self, block: u32, data: &RRef<[u8; 512]>);
}