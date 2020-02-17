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

fn get_path(p: &syn::Path) -> String {
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
                        syn::GenericArgument::Type(nt) => t += &get_type(nt),
                        _ => {println!("[ERROR] Non-type generic arguments not supported"); panic!()}
                    }
                }
                t += ">"
            },
            _ => {println!("[ERROR] Must be angle-bracketed generic"); panic!()}
        }
        colon = true;
    }
    t
}

fn get_type(ty: &syn::Type) -> String {
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
            t += &get_type(&*r.elem)
        },
        syn::Type::Array(a) => {
            t += "[";
            t += &get_type(&*a.elem);
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
            t += &get_path(&p.path)
        },
        syn::Type::Tuple(tup) => {
            t += "(";
            let mut comma = false;
            for item in &tup.elems {
                if comma {
                    t += ", "
                }
                comma = true;
                t += &get_type(&item)
            }
            t += ")"
        },
        syn::Type::TraitObject(tr) => {
            match &tr.dyn_token {
                Some(_) => t += "dyn ",
                None => ()
            }
            if tr.bounds.len() != 1 {
                println!("[ERROR] Multiple traits not allowed");
                panic!()
            }
            match &tr.bounds[0] {
                syn::TypeParamBound::Trait(tr) => t += &get_path(&tr.path),
                _ => {println!("[ERROR] Must be a trait"); panic!()}
            }
        },
        syn::Type::Slice(s) => {
            t += "[";
            t += &get_type(&*s.elem);
            t += "]"
        },
        _ => {println!("[ERROR] This type not allowed"); panic!()}
    }
    t
}

fn process_typed(typed: syn::PatType) -> Arg {
    match *typed.pat {
        syn::Pat::Ident(id) => Arg {name: id.ident.to_string(), typ: get_type(&*typed.ty)},
        _ => panic!()
    }
}

fn process_arg(arg: syn::FnArg) -> Arg {
    match arg {
        syn::FnArg::Typed(a) => process_typed(a),
        syn::FnArg::Receiver(_) => Arg {name: "self".to_string(), typ: String::new()}
    }
}

fn extract(ast: syn::File) -> (Vec<Scope>, Vec<Scope>, Vec<Arg>) {
    let whitelist = vec!["Syscall", "Heap"];
    let mut args: Vec<Arg> = Vec::new();
    let mut funcs: Vec<Scope> = Vec::new();
    let mut traits: Vec<Scope> = Vec::new();
    for item in ast.items {
        let tr = match item {
            syn::Item::Trait(tr) => tr,
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
                args.push(process_arg(item))
            }
            let rt = match func.sig.output {
                syn::ReturnType::Type(_, ty) => get_type(&ty),
                _ => "()".to_string()
            };
            args.push(Arg {name: "__rt".to_string(), typ: rt});
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
                println!("\t\t{}: {}", args[j].name, args[j].typ)
            }
        }
    }
}