pub const BUILD_VERSION: Option<&'static str> = option_env!("BUILD_VERSION");
pub const INTERFACE_FINGERPRINT: Option<&'static str> = option_env!("INTERFACE_FINGERPRINT");
pub const TRUSTED_SIGNING_KEY: &'static [u8] = include_bytes!("../redleaf.pub");
