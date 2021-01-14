use fs::SuperBlock;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print!("Usage: mkfs fs.img files...\n");
    }
}