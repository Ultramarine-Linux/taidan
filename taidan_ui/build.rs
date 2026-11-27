use slint_build::CompilerConfiguration;

fn main() {
    let cfg = CompilerConfiguration::new().with_style("fluent".into()).with_edition((1, 19));
    slint_build::compile_with_config("../ui/app.slint", cfg).unwrap();
}
