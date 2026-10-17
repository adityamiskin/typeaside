fn main() {
    println!("cargo:rerun-if-changed=packaging/windows/typeaside.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("packaging/windows/typeaside.ico")
            .compile()
            .expect("compile TypeAside icon resource");
    }
}
