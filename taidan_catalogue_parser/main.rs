fn main() {
    let mut args = std::env::args();
    let bin = args.next().unwrap();
    if args.len() == 0 {
        eprintln!("Usage: {bin} <path/to/yaml> <path/to/another/yaml> ...");
        return;
    }
    for p in args {
        taidan_catalogue_parser::Category::parse_path(std::path::Path::new(&p)).unwrap();
    }
}
