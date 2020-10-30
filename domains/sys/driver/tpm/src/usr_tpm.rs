macro_rules! generate_tpm {
    (fn $func:ident(self, $($arg:ident : $ty:ty),*) -> $ret:ty) => {
        pub struct UsrTpm {
            tpm: ::alloc::boxed::Box<dyn ::usr::tpm::TpmDev>,
        }
    
        // impl usr::tpm::UsrTpm for Tpm {
    
        // }

        impl UsrTpm {
            fn $func(self, $($arg: $ty,)*) -> $ret {
                ::libtpm::$func(&*self.tpm, $($arg), *)
            }
       }
    };
}

generate_tpm!(
    fn tpm_validate_locality(self, locality: u32) -> bool
);