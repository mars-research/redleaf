use x86::cpuid::CpuId;

pub fn boot_banner() {
    let cpuid = CpuId::new();
    match cpuid.get_vendor_info() {
        Some(vendor) => println!("RedLeaf booting on {}", vendor.as_string()),
        None => println!("RedLeaf"),
    }
}
