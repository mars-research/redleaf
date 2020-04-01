use std::fs::File;
use std::io::Read;
use std::env;
use proc_macro2;

struct Scope {
    name: String,
    start: usize,
    end: usize
}

struct Arg {
    name: String,
    ty: String
}

fn get_path(p: &syn::Path, known: &Vec<String>) -> String {
    let mut t = String::new();
    let mut colon = match p.leading_colon {
        Some(_) => true,
        None => false
    };
    for seg in &p.segments {
        if colon {
            t += "::";
        }
        t += &seg.ident.to_string();
        match &seg.arguments {
            syn::PathArguments::None => (),
            syn::PathArguments::AngleBracketed(args) => {
                t += "<";
                let mut comma = false;
                for arg in &args.args {
                    if comma {
                        t += ", "
                    }
                    comma = true;
                    match arg {
                        syn::GenericArgument::Type(nt) => t += &get_type(nt, false, known),
                        _ => {println!("[ERROR] Non-type generic arguments not supported"); panic!()}
                    }
                }
                t += ">"
            },
            _ => {println!("[ERROR] Must be angle-bracketed generic"); panic!()}
        }
        colon = true;
    }
    let l = p.segments.len();
    let last = &p.segments[l - 1];
    let mut found = false;
    for k in known {
        if *k == last.ident.to_string() {
            found = true;
            break;
        }
    }
    if !found {
        println!("[WARNING] Unrecognized type {}", last.ident)
    }
    t
}

fn get_type(ty: &syn::Type, top: bool, known: &Vec<String>) -> String {
    let mut t = String::new();
    match ty {
        syn::Type::Reference(r) => {
            t += "&";
            match &r.lifetime {
                Some(v) => {t += "'"; t += &v.ident.to_string(); t += " "},
                None => ()
            }
            match r.mutability {
                Some(_) => t += "mut ",
                None => ()
            }
            t += &get_type(&*r.elem, false, known)
        },
        syn::Type::Array(a) => {
            t += "[";
            t += &get_type(&*a.elem, false, known);
            t += "; ";
            t += &match &a.len {
                syn::Expr::Lit(l) => match &l.lit {
                    syn::Lit::Int(i) => i,
                    _ => panic!()
                },
                _ => {println!("[ERROR] Must be literal size"); panic!()}
            }.to_string();
            t += "]"
        },
        syn::Type::Path(p) => {
            t += &get_path(&p.path, known)
        },
        syn::Type::Tuple(tup) => {
            if top {
                println!("[ERROR] This type not allowed here")
            }

            t += "(";
            let mut comma = false;
            for item in &tup.elems {
                if comma {
                    t += ", "
                }
                comma = true;
                t += &get_type(&item, false, known)
            }
            t += ")"
        },
        syn::Type::TraitObject(tr) => {
            if top {
                println!("[ERROR] This type not allowed here")
            }

            match &tr.dyn_token {
                Some(_) => t += "dyn ",
                None => ()
            }
            if tr.bounds.len() != 1 {
                println!("[ERROR] Multiple traits not allowed");
                panic!()
            }
            match &tr.bounds[0] {
                syn::TypeParamBound::Trait(tr) => t += &get_path(&tr.path, known),
                _ => {println!("[ERROR] Must be a trait"); panic!()}
            }
        },
        syn::Type::Slice(s) => {
            if top {
                println!("[ERROR] This type not allowed here")
            }

            t += "[";
            t += &get_type(&*s.elem, false, known);
            t += "]"
        },
        _ => {println!("[ERROR] This type not allowed"); panic!()}
    }
    t
}

fn process_typed(typed: syn::PatType, known: &Vec<String>) -> Arg {
    match *typed.pat {
        syn::Pat::Ident(id) => Arg {name: id.ident.to_string(), ty: get_type(&*typed.ty, true, known)},
        _ => panic!()
    }
}

fn process_arg(arg: syn::FnArg, known: &Vec<String>) -> Arg {
    match arg {
        syn::FnArg::Typed(a) => process_typed(a, known),
        syn::FnArg::Receiver(_) => Arg {name: "self".to_string(), ty: String::new()}
    }
}

fn is_copy(tokens: proc_macro2::TokenStream) -> bool {
    for tok in tokens {
        match tok {
            proc_macro2::TokenTree::Group(g) => {
                for tok in g.stream() {
                    match tok {
                        proc_macro2::TokenTree::Ident(id) => {
                            if id == "Copy" {
                                return true
                            }
                        },
                        _ => ()
                    }
                }
            },
            _ => panic!()
        }
    }
    false
}

fn process_attrs(attrs: Vec<syn::Attribute>) -> bool {
    for attr in attrs {
        if attr.path.segments.len() == 1 && attr.path.segments[0].ident == "derive" {
            return is_copy(attr.tokens);
        }
    }
    false
}

fn extract(ast: syn::File) -> (Vec<Scope>, Vec<Scope>, Vec<Arg>) {
    let whitelist = vec!["Syscall", "Heap"];
    let mut args: Vec<Arg> = Vec::new();
    let mut funcs: Vec<Scope> = Vec::new();
    let mut traits: Vec<Scope> = Vec::new();
    let mut known: Vec<String> = Vec::new();
    for item in ast.items {
        let tr = match item {
            syn::Item::Trait(tr) => {
                println!("Found trait {}", tr.ident);
                known.push(tr.ident.to_string());
                tr
            },
            syn::Item::Enum(e) => {
                println!("Found enum {}", e.ident);
                if process_attrs(e.attrs) {
                    known.push(e.ident.to_string());
                }
                continue
            },
            syn::Item::Struct(e) => {
                println!("Found struct {}", e.ident);
                if process_attrs(e.attrs) {
                    known.push(e.ident.to_string());
                }
                continue
            },
            syn::Item::TraitAlias(e) => {
                println!("Found trait alias {}", e.ident);
                known.push(e.ident.to_string());
                continue
            },
            syn::Item::Type(e) => {
                println!("Found type alias {}", e.ident);
                known.push(e.ident.to_string());
                continue
            },
            syn::Item::Use(_) => {
                println!("Found use decl (WARNING: incomplete handling)");
                continue
            },
            _ => continue
        };

        let mut trscope = Scope {name: tr.ident.to_string(), start: funcs.len(), end: 0};

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
            let mut fscope = Scope {name: func.sig.ident.to_string(), start: args.len(), end: 0};
            for item in func.sig.inputs {
                args.push(process_arg(item, &known))
            }
            let rt = match func.sig.output {
                syn::ReturnType::Type(_, ty) => get_type(&ty, true, &known),
                _ => "()".to_string()
            };
            args.push(Arg {name: "__rt".to_string(), ty: rt});
            fscope.end = args.len();
            funcs.push(fscope)
        }

        trscope.end = funcs.len();
        traits.push(trscope);
    }
    (traits, funcs, args)
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
    let (traits, funcs, args) = extract(ast);
    for tr in traits {
        println!("{}", tr.name);
        for i in tr.start .. tr.end {
            println!("\t{}", funcs[i].name);
            for j in funcs[i].start .. funcs[i].end {
                println!("\t\t{}: {}", args[j].name, args[j].ty)
            }
        }
    }
}