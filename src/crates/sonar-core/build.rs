extern crate napi_build;

fn main() {
    napi_build::setup();

    // Linux ships transcribe-cpp as a shared libtranscribe + dlopen'd ggml
    // backend modules (the `dynamic-backends` posture in Cargo.toml). Bake an
    // $ORIGIN-relative rpath into the compiled `.node` addon so it finds those
    // sibling libraries in its own directory (napi build drops the `.node`
    // file in the crate root, right where `stage_transcribe_runtime_libs`
    // below copies them). Windows resolves DLLs via `SetDllDirectory` at
    // runtime instead (see `init_transcribe_backend` in src/lib.rs); macOS
    // links transcribe-cpp statically via the `metal` feature, so neither
    // step applies there.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
    }

    stage_transcribe_runtime_libs();
}

/// Copy transcribe-cpp's shared runtime libraries (and the dlopen'd ggml
/// backend modules) into the crate root, next to where `napi build` places
/// the compiled `.node` addon. Self-gates on the shared / dynamic-backends
/// posture used by Windows x86_64 and Linux; it's a no-op for the static
/// macOS `metal` build, where there is nothing to ship.
///
/// Ported from Handy's `stage_transcribe_runtime_libs` (src-tauri/build.rs).
fn stage_transcribe_runtime_libs() {
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    println!("cargo:rerun-if-env-changed=DEP_TRANSCRIBE_CPP_RUNTIME_DIR");
    println!("cargo:rerun-if-env-changed=DEP_TRANSCRIBE_CPP_MODULE_DIR");

    // Present only in a shared posture. A static build has nothing to ship.
    let Some(runtime_dir) = std::env::var_os("DEP_TRANSCRIBE_CPP_RUNTIME_DIR") else {
        return;
    };

    // transcribe-cpp publishes its runtime layout in up to two directories:
    //   RUNTIME_DIR : the shared libs to load (transcribe + core ggml / ggml-base)
    //   MODULE_DIR  : the dlopen'd ggml backend modules (per-ISA ggml-cpu-* and
    //                 ggml-vulkan), dynamic-backends only. Often the same dir.
    // BOTH must be discoverable at runtime, or init_backends_default() finds
    // the core libs but zero loadable compute backends.
    let mut dirs = BTreeSet::new();
    dirs.insert(PathBuf::from(runtime_dir));
    if let Some(module_dir) = std::env::var_os("DEP_TRANSCRIBE_CPP_MODULE_DIR") {
        dirs.insert(PathBuf::from(module_dir));
    }

    // The crate root — where package.json's `napi build` writes the compiled
    // `.node` file and `index.js`. Keeping the libs here (rather than a
    // `transcribe-libs/` subfolder) means the rpath / SetDllDirectory target
    // is simply "wherever the addon itself lives", dev or packaged.
    let dest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());

    let mut libs: std::collections::BTreeMap<String, PathBuf> = Default::default();
    for dir in &dirs {
        println!("cargo:rerun-if-changed={}", dir.display());
        for entry in std::fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
            .flatten()
        {
            let src = entry.path();
            let name = src.file_name().and_then(|s| s.to_str()).unwrap_or("");
            // Match by NAME, not extension: Linux versions its libs
            // (libtranscribe.so.0, .so.0.1.3), so an extension-only filter
            // would miss the versioned names entirely.
            let is_lib = name.ends_with(".dll")
                || name.ends_with(".dylib")
                || name.ends_with(".so")
                || name.contains(".so.");
            if is_lib {
                libs.insert(name.to_string(), src);
            }
        }
    }

    // Linux carries each lib as a symlink chain (libfoo.so -> libfoo.so.0 ->
    // libfoo.so.0.1.3); stage exactly one name per lib — the SONAME
    // (`libfoo.so.N`) for core libs (what NEEDED entries reference), the bare
    // unversioned name for the dlopen'd ggml modules — to avoid triplicating
    // each lib on disk. `fs::copy` dereferences the symlink either way.
    let mut best: std::collections::BTreeMap<&str, (&str, &PathBuf, usize)> = Default::default();
    for (name, src) in &libs {
        let (stem, rank) = match split_versioned_so(name) {
            None => (name.as_str(), 0), // Windows/macOS: unversioned, keep as-is.
            Some((stem, depth)) => (stem, if depth == 1 { 0 } else { depth + 1 }),
        };
        match best.get(stem) {
            Some(&(_, _, existing)) if existing <= rank => {}
            _ => {
                best.insert(stem, (name, src, rank));
            }
        }
    }

    let mut copied = 0usize;
    for &(name, src, _) in best.values() {
        let dest_path = dest.join(name);
        // Skip the copy if the destination is already byte-identical — avoids
        // needlessly rewriting (and re-triggering downstream watchers on) a
        // file that hasn't changed between incremental builds.
        let unchanged = std::fs::metadata(&dest_path)
            .and_then(|d| std::fs::metadata(src).map(|s| d.len() == s.len()))
            .unwrap_or(false);
        if !unchanged {
            std::fs::copy(src, &dest_path).unwrap_or_else(|e| panic!("copy {}: {e}", src.display()));
        }
        copied += 1;
    }
    if copied == 0 {
        panic!(
            "no transcribe-cpp runtime libraries found under {dirs:?}; a shared / \
             dynamic-backends build must ship them or the app registers zero \
             compute devices"
        );
    }
    println!("cargo:warning=Staged {copied} transcribe-cpp runtime library file(s) into crate root");
}

/// Split a versioned ELF shared-library name into (stem, version depth):
/// `libfoo.so` -> ("libfoo", 0), `libfoo.so.0` -> ("libfoo", 1),
/// `libfoo.so.0.1.3` -> ("libfoo", 3). Returns None for names that aren't a
/// `.so` optionally followed by dot-separated numeric components.
fn split_versioned_so(name: &str) -> Option<(&str, usize)> {
    let idx = name.find(".so")?;
    let (stem, rest) = (&name[..idx], &name[idx + 3..]);
    if rest.is_empty() {
        return Some((stem, 0));
    }
    let comps: Vec<&str> = rest.strip_prefix('.')?.split('.').collect();
    comps
        .iter()
        .all(|c| !c.is_empty() && c.bytes().all(|b| b.is_ascii_digit()))
        .then_some((stem, comps.len()))
}
