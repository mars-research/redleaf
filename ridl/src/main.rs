use std::fs::File;
use std::io::Read;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: <invocation> <filepath>");
        return
    }
    let mut file = match File::open(&args[1]) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e);
            return
        }
    };
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();
    let ast = syn::parse_file(&content).unwrap();
    let mut traits : Vec<syn::ItemTrait> = Vec::new();
    for item in ast.items {
        match item {
            syn::Item::Trait(tr) => traits.push(tr),
            _ => ()
        }
    }
    for tr in traits {
        let name = tr.ident.to_string();
        for item in tr.items {
            match item {
                syn::TraitItem::Method(m) => println!("{}::{}", name, m.sig.ident),
                _ => ()
            }
        }
    }
}
