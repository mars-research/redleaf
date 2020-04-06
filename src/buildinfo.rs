pub const BUILD_VERSION: &'static str = env!("BUILD_VERSION");
pub const INTERFACE_FINGERPRINT: &'static str = env!("INTERFACE_FINGERPRINT");
pub const TRUSTED_SIGNING_KEY: &'static [u8] = include_bytes!("../redleaf.pub");
