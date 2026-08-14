fn main() {
    println!("cargo:rerun-if-env-changed=DEP_TRANSCRIBE_CPP_RUNTIME_DIR");
    println!("cargo:rerun-if-env-changed=DEP_TRANSCRIBE_CPP_MODULE_DIR");

    if let Some(dir) = std::env::var_os("DEP_TRANSCRIBE_CPP_RUNTIME_DIR") {
        println!("cargo:runtime_dir={}", dir.to_string_lossy());
    }
    if let Some(dir) = std::env::var_os("DEP_TRANSCRIBE_CPP_MODULE_DIR") {
        println!("cargo:module_dir={}", dir.to_string_lossy());
    }
}
