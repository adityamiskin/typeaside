fn main() {
    println!("cargo:rerun-if-changed=packaging/windows/typeaside.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        // Whisper's Metal availability checks need Clang's runtime on older macOS.
        let output = std::process::Command::new("xcrun")
            .args(["clang", "--print-resource-dir"])
            .output()
            .expect("locate Clang runtime");
        assert!(output.status.success(), "locate Clang runtime");
        let resource_dir =
            String::from_utf8(output.stdout).expect("Clang resource directory is UTF-8");
        let runtime_dir = std::path::Path::new(resource_dir.trim()).join("lib/darwin");
        assert!(
            runtime_dir.join("libclang_rt.osx.a").is_file(),
            "Clang macOS runtime is missing"
        );
        println!("cargo:rustc-link-search=native={}", runtime_dir.display());
        println!("cargo:rustc-link-lib=static=clang_rt.osx");
    }
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("packaging/windows/typeaside.ico")
            .compile()
            .expect("compile TypeAside icon resource");
    }
}
