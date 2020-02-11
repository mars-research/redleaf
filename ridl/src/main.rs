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
        println!("{}", tr.ident);
        for item in tr.items {
            let method = match item {
                syn::TraitItem::Method(m) => m,
                _ => continue
            };
            println!("\t{}", method.sig.ident);
            for arg in method.sig.inputs {
                match arg {
                    syn::FnArg::Typed(a) => println!("\t\t{}", match *a.pat {
                        syn::Pat::Ident(id) => id.ident,
                        _ => continue
                    }),
                    syn::FnArg::Receiver(r) => {
                        print!("\t\t");
                        match r.reference {
                            Some(t) => {
                                print!("&");
                                match t.1 {
                                    Some(l) => print!("{} ", l),
                                    None => ()
                                }
                            }
                            None => ()
                        }
                        match r.mutability {
                            Some(_) => print!("mut "),
                            None => ()
                        }
                        println!("self")
                    }
                }
            }
        }
    }
}
