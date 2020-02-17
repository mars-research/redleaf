use std::fs::File;
use std::io::Read;
use std::env;

struct Scope {
    name: String,
    start: usize,
    end: usize
}

struct Arg {
    name: String,
    typ: String
}

fn get_type(ty: syn::Type) -> String {
    let mut t = String::new();
    match ty {
        syn::Type::Reference(r) => {
            t += "&";
            match r.lifetime {
                Some(v) => {t += "'"; t += &v.ident.to_string(); t += " "},
                None => ()
            }
            match r.mutability {
                Some(_) => t += "mut ",
                None => ()
            }
            t += &get_type(*r.elem)
        },
        syn::Type::Array(a) => {
            t += "[";
            t += &get_type(*a.elem);
            t += "; ";
            t += &match a.len {
                syn::Expr::Lit(l) => match l.lit {
                    syn::Lit::Int(i) => i,
                    _ => panic!()
                },
                _ => {println!("NOT LITERAL"); panic!()}
            }.to_string();
            t += "]"
        },
        syn::Type::Path(p) => {
            let mut colon = match p.path.leading_colon {
                Some(_) => true,
                None => false
            };
            for seg in p.path.segments {
                if colon {
                    t += "::";
                }
                t += &seg.ident.to_string();
                match seg.arguments {
                    syn::PathArguments::None => (),
                    syn::PathArguments::AngleBracketed(args) => {
                        t += "<";
                        let mut comma = false;
                        for arg in args.args {
                            if comma {
                                t += ", "
                            }
                            comma = true;
                            match arg {
                                syn::GenericArgument::Type(nt) => t += &get_type(nt),
                                _ => {println!("NOT A TYPE"); panic!()}
                            }
                        }
                        t += ">"
                    },
                    _ => {println!("NOT AB"); panic!()}
                }
                colon = true;
            }
        },
        syn::Type::Tuple(_) => print!("Strange Type: "),
        syn::Type::TraitObject(_) => print!("Strange Type: "),
        syn::Type::Slice(_) => print!("Strange Type: "),
        _ => {println!("ILLEGAL TYPE!!"); panic!()}
    }
    t
}

fn process_typed(typed: syn::PatType) -> Arg {
    match *typed.pat {
        syn::Pat::Ident(id) => Arg {name: id.ident.to_string(), typ: get_type(*typed.ty)},
        _ => panic!()
    }
}

fn process_arg(arg: syn::FnArg) -> Arg {
    match arg {
        syn::FnArg::Typed(a) => process_typed(a),
        syn::FnArg::Receiver(_) => Arg {name: "self".to_string(), typ: String::new()}
    }
}

fn extract(ast: syn::File) {
    let whitelist = vec!["Syscall", "Heap"];
    for item in ast.items {
        let tr = match item {
            syn::Item::Trait(tr) => tr,
            _ => continue
        };
        let mut in_white = false;
        for w in &whitelist {
            if tr.ident.to_string() == *w {
               in_white = true;
               break
            }
        }
        if in_white {
            continue
        }
        for item in tr.items {
            let func = match item {
                syn::TraitItem::Method(m) => m,
                _ => continue
            };
            for item in func.sig.inputs {
                let arg = process_arg(item);
                println!("{} {}", arg.name, arg.typ)
            }
        }
    }
}

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
    extract(ast);
}