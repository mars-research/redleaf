#![feature(log_syntax, proc_macro_def_site)]

use core::panic;
use std::collections::HashMap;
use quote::{format_ident, quote};

use lazy_static::lazy_static;
use proc_macro::{Ident, TokenStream};
use syn::{ImplItem, ImplItemMethod, ItemStruct, parse_quote};

lazy_static!(
    static ref SIZE_MAP: HashMap<&'static str, usize> = vec![
        ("u8", 1),
        ("u16", 2),
        ("u32", 4),
        ("u64", 8),
        ("i8", 1),
        ("i16", 2),
        ("i32", 4),
        ("i64", 8),

    ].into_iter().collect();
);

/// Generate a 
#[proc_macro_attribute]
pub fn volatile_accessor(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    assert!(attr.is_empty(), &attr.to_string());
    let st: ItemStruct = syn::parse(item).expect("interface definition must be a valid struct definition");

    assert_eq!(st.generics.params.len(), 0, "Generic is not supported: {:?}", st);

    let mut offset: usize = 0;
    let mut accessor_impls: Vec<ImplItemMethod> = vec![];
    for field in &st.fields {
        match &field.ty {
            syn::Type::Array(x) => unimplemented!("{:?}", x),
            syn::Type::BareFn(x) => unimplemented!("{:?}", x),
            syn::Type::Group(x) => unimplemented!("{:?}", x),
            syn::Type::ImplTrait(x) => unimplemented!("{:?}", x),
            syn::Type::Infer(x) => unimplemented!("{:?}", x),
            syn::Type::Macro(x) => unimplemented!("{:?}", x),
            syn::Type::Never(x) => unimplemented!("{:?}", x),
            syn::Type::Paren(x) => unimplemented!("{:?}", x),
            syn::Type::Ptr(x) => unimplemented!("{:?}", x),
            syn::Type::Reference(x) => unimplemented!("{:?}", x),
            syn::Type::Slice(x) => unimplemented!("{:?}", x),
            syn::Type::TraitObject(x) => unimplemented!("{:?}", x),
            syn::Type::Tuple(x) => unimplemented!("{:?}", x),
            syn::Type::Verbatim(x) => unimplemented!("{:?}", x),
            syn::Type::__Nonexhaustive => unimplemented!(),
            syn::Type::Path(path) => {
                let field_ident = field.ident.as_ref().expect(&format!("All field must be named: {:?}", st));
                let field_type = &field.ty;
                let path = path.path.segments.iter().map(|seg| seg.ident.to_string()).collect::<Vec<String>>().join("::");
                let size = SIZE_MAP.get(path.as_str()).expect(&format!("Type {} not supported. Supported types are {:?}", path, *SIZE_MAP));
                let read_accessor_ident = format_ident!("read_{}", &field_ident);
                let write_accessor_ident = format_ident!("write_{}", &field_ident);
                
                // // Generate read and write accessors.
                // panic!("{:#?}", quote!(
                //     // fn #read_accessor_ident(&self) -> #path {
                //     //     ::core::ptr::read_volatile((self.base + #offset) as *const #path)
                //     // }
                // ));

                accessor_impls.push(parse_quote! {
                    pub fn #read_accessor_ident(&self) -> #field_type {
                        unsafe { ::core::ptr::read_volatile((self.base + #offset) as *const #field_type) }
                    }
                });
                accessor_impls.push(parse_quote! {
                    pub fn #write_accessor_ident(&self, value: #field_type) {
                        unsafe { ::core::ptr::write_volatile((self.base + #offset) as *const #field_type as *mut #field_type, value) }
                    }
                });
                offset += size;
            },
        }
    }

    
    let accessor_ident = format_ident!("{}VolatileAccessor", st.ident);
    let vis = &st.vis;
    TokenStream::from(quote! {
        #st

        #[allow(dead_code)]
        #vis struct #accessor_ident {
            base: usize,    
        }

        #[allow(dead_code)]
        impl #accessor_ident {
            pub unsafe fn new(base: usize) -> Self {
                Self {
                    base,
                }
            }

            #(#accessor_impls)*
        }

    })
}
