use syntex_syntax::*;
use parse::*;
use codemap::*;
use std::vec::Vec;
use std::path::Path;

fn main() {
    let sess = ParseSess::new(FilePathMapping::new(Vec::new()));
    let path = Path::new("../rtool/test.rs");
    let mut parser = new_parser_from_file(&sess, &path);
    match parser.parse_crate_mod() {
        Ok(v) => println!("{:?}", v),
        Err(e) => println!("{:?}", e)
    };
}
