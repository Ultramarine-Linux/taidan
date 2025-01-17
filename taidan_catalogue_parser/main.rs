fn main() -> color_eyre::Result<()> {
    color_eyre::install().unwrap();
    let mut args = std::env::args();
    let bin = args.next().unwrap();
    if args.len() == 0 {
        eprintln!("Usage: {bin} <path/to/yaml> <path/to/another/yaml> ...");
        std::process::exit(1);
    }
    for p in args {
        print!(" Check: {p}\r");
        taidan_catalogue_parser::Category::parse_path(std::path::Path::new(&p))
            .map_err(|e| color_eyre::Report::msg(format!("cannot parse {p}")).wrap_err(e))?;
        println!(" PASS : {p}");
    }
    Ok(())
}
