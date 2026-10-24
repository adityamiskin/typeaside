use std::{env, path::PathBuf};

fn main() {
    let include = env::var("DEP_TRANSCRIBE_INCLUDE_DIR")
        .expect("transcribe-cpp-sys 0.4.1 must export its installed ggml headers");
    assert!(PathBuf::from(&include).join("ggml.h").is_file());
    println!("cargo:rerun-if-changed=llama");
    println!("cargo:rerun-if-changed=CMakeLists.txt");
    let mut config = cmake::Config::new(".");
    config
        .profile("Release")
        .define("TRANSCRIBE_INCLUDE_DIR", &include);
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        config.cxxflag("/O2").cxxflag("/DNDEBUG");
    }
    let dst = config.build();
    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-lib=static=llama");
    let bindings = bindgen::Builder::default()
        .header("llama/include/llama.h")
        .clang_arg(format!("-I{include}"))
        .allowlist_function("llama_(log_set|backend_init|model_default_params|context_default_params|model_load_from_file|model_free|init_from_model|free|model_get_vocab|get_memory|memory_clear|memory_seq_rm|tokenize|batch_get_one|decode|sampler_init_greedy|sampler_sample|sampler_free|vocab_is_eog|token_to_piece)")
        .layout_tests(false)
        .generate_comments(false)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("generate pinned llama bindings");
    bindings
        .write_to_file(PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("bindings.rs"))
        .expect("write llama bindings");
}
