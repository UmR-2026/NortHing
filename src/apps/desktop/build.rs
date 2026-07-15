fn main() {
    // Compile the main slint file and all its imports
    slint_build::compile_with_config(
        "src/ui/main.slint",
        slint_build::CompilerConfiguration::new().with_style("material".into()),
    )
    .unwrap();

    println!("cargo:rerun-if-changed=src/ui/main.slint");
    println!("cargo:rerun-if-changed=src/ui/components");
    println!("cargo:rerun-if-changed=src/ui/views");
}
