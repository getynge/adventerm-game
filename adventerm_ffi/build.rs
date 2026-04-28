fn main() {
    let crate_dir = env!("CARGO_MANIFEST_DIR");
    let config = cbindgen::Config::from_file(format!("{crate_dir}/cbindgen.toml")).unwrap();
    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(config)
        .generate()
        .expect("cbindgen failed")
        .write_to_file(format!("{crate_dir}/include/adventerm_ffi.h"));
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=cbindgen.toml");
}
