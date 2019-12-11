use core::collections::BTreeMap;
use core::ops::Range;

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    pub drivers: Vec<DriverConfig>
}

#[derive(Debug, Default, Deserialize)]
pub struct DriverConfig {
    pub name: Option<String>,
    pub class: Option<u8>,
    pub subclass: Option<u8>,
    pub interface: Option<u8>,
    pub ids: Option<BTreeMap<String, Vec<u16>>>,
    pub vendor: Option<u16>,
    pub device: Option<u16>,
    pub device_id_range: Option<Range<u16>>,
    pub command: Option<Vec<String>>
}
