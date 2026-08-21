use std::{collections::BTreeSet, env, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-env-changed=DEP_SONAR_TRANSCRIPTION_RUNTIME_DIR");
    println!("cargo:rerun-if-env-changed=DEP_SONAR_TRANSCRIPTION_MODULE_DIR");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
    }

    let Some(runtime_dir) = env::var_os("DEP_SONAR_TRANSCRIPTION_RUNTIME_DIR") else {
        return Ok(());
    };
    let mut sources = BTreeSet::from([PathBuf::from(runtime_dir)]);
    if let Some(module_dir) = env::var_os("DEP_SONAR_TRANSCRIPTION_MODULE_DIR") {
        sources.insert(PathBuf::from(module_dir));
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let binary_dir = out_dir
        .ancestors()
        .nth(3)
        .ok_or("Cargo OUT_DIR did not contain a profile directory")?;
    for source_dir in sources {
        for entry in fs::read_dir(source_dir)? {
            let source = entry?.path();
            let name = source
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            let is_library = source.extension().is_some_and(|extension| {
                extension.eq_ignore_ascii_case("dll")
                    || extension.eq_ignore_ascii_case("dylib")
                    || extension.eq_ignore_ascii_case("so")
            }) || name.contains(".so.");
            if is_library {
                fs::copy(&source, binary_dir.join(name))?;
            }
        }
    }
    Ok(())
}
