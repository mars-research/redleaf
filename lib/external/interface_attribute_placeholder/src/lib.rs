///! Include multiple attributes that 
use proc_macro::TokenStream;

macro_rules! generate_placeholder_attributes {
    () => {};
    ($attr:ident) => {
        #[doc = "Placeholder attribute; noop besides removing the attribute itself."] 
        #[proc_macro_attribute]
        pub fn $attr(_attr: TokenStream, item: TokenStream) -> TokenStream {
            item
        }
    };
    ($attr:ident, $($attrs:tt)*) => {
        generate_placeholder_attributes!($attr);
        generate_placeholder_attributes!($($attrs)*);
    };
}

generate_placeholder_attributes! {
    placeholder,
    interface,
    domain_create,
    domain_create_blob,
}
// #[proc_macro_attribute]
// pub fn a(mut attr: TokenStream, item: TokenStream) -> TokenStream {
//     let attr = proc_macro2::TokenStream::from(attr);
//     let item = proc_macro2::TokenStream::from(item);
//     let output = quote::quote! {
//         #[$attr(#attr)]
//         #item
//     };
//     output.into()
// }