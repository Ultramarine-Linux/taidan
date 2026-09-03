use slint_build::CompilerConfiguration;

fn main() {
    // glib_build_tools::compile_resources(&["data"], "data/icons.gresource.xml", "icons.gresource");
    let cfg = CompilerConfiguration::new().with_style("fluent-light".into());
    slint_build::compile_with_config("ui/app.slint", cfg).unwrap();
}
