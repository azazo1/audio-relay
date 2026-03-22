use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    for key in [
        "OPUS_DIR",
        "OPUS_LIB_DIR",
        "OPUS_LIB_NAME",
        "VCPKG_ROOT",
        "VCPKG_INSTALLATION_ROOT",
        "VCPKG_DEFAULT_TRIPLET",
    ] {
        println!("cargo:rerun-if-env-changed={key}");
    }

    let target = env::var("TARGET").unwrap_or_default();
    link_opus(&target);

    if target.contains("apple-darwin") {
        build_macos_native();
    }
}

fn link_opus(target: &str) {
    if let Some((dir, lib_name)) = manual_opus_link() {
        println!("cargo:rustc-link-search=native={}", dir.display());
        println!("cargo:rustc-link-lib={lib_name}");
        return;
    }

    if target.contains("windows") {
        link_opus_windows(target);
    } else {
        link_opus_pkg_config();
    }
}

fn manual_opus_link() -> Option<(PathBuf, String)> {
    if let Some(dir) = env::var_os("OPUS_LIB_DIR") {
        let lib_name = env::var("OPUS_LIB_NAME").unwrap_or_else(|_| "opus".to_string());
        return Some((PathBuf::from(dir), lib_name));
    }

    let opus_dir = env::var_os("OPUS_DIR")?;
    let lib_name = env::var("OPUS_LIB_NAME").unwrap_or_else(|_| "opus".to_string());
    let base = PathBuf::from(opus_dir);
    for candidate in [base.join("lib"), base.join("lib64"), base.clone()] {
        if candidate.is_dir() {
            return Some((candidate, lib_name));
        }
    }

    None
}

fn link_opus_windows(target: &str) {
    let default_lib_name = env::var("OPUS_LIB_NAME").unwrap_or_else(|_| "opus".to_string());
    if let Some((dir, lib_name)) = find_vcpkg_opus(target, &default_lib_name) {
        println!("cargo:rustc-link-search=native={}", dir.display());
        println!("cargo:rustc-link-lib={lib_name}");
        return;
    }

    println!("cargo:rustc-link-lib={default_lib_name}");
    println!(
        "cargo:warning=audio-relay could not locate an Opus import/static library automatically; set OPUS_LIB_DIR/OPUS_LIB_NAME or VCPKG_ROOT if link fails"
    );
}

fn find_vcpkg_opus(target: &str, default_lib_name: &str) -> Option<(PathBuf, String)> {
    let root = env::var_os("VCPKG_ROOT")
        .or_else(|| env::var_os("VCPKG_INSTALLATION_ROOT"))
        .map(PathBuf::from)?;

    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let target_features = env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();
    let mut triplets = Vec::new();
    if let Ok(explicit) = env::var("VCPKG_DEFAULT_TRIPLET") {
        triplets.push(explicit);
    }
    triplets.push(default_vcpkg_triplet(target, target_features.contains("crt-static")));
    triplets.push("x64-windows".to_string());
    triplets.push("x64-windows-static".to_string());
    triplets.dedup();

    let preferred_debug = profile == "debug";
    let lib_names = candidate_windows_lib_names(default_lib_name);

    for triplet in triplets {
        let installed = root.join("installed").join(&triplet);
        let mut search_dirs = Vec::new();
        if preferred_debug {
            search_dirs.push(installed.join("debug").join("lib"));
            search_dirs.push(installed.join("lib"));
        } else {
            search_dirs.push(installed.join("lib"));
            search_dirs.push(installed.join("debug").join("lib"));
        }

        for dir in search_dirs {
            if !dir.is_dir() {
                continue;
            }

            for lib_name in &lib_names {
                let file = dir.join(format!("{lib_name}.lib"));
                if file.is_file() {
                    return Some((dir, lib_name.clone()));
                }
            }
        }
    }

    None
}

fn candidate_windows_lib_names(default_lib_name: &str) -> Vec<String> {
    let mut names = vec![
        default_lib_name.to_string(),
        format!("{default_lib_name}d"),
        format!("lib{default_lib_name}"),
        format!("lib{default_lib_name}d"),
        format!("{default_lib_name}_static"),
        format!("lib{default_lib_name}_static"),
    ];
    names.dedup();
    names
}

fn default_vcpkg_triplet(target: &str, crt_static: bool) -> String {
    let arch = if target.starts_with("x86_64") {
        "x64"
    } else if target.starts_with("aarch64") {
        "arm64"
    } else if target.starts_with("i686") || target.starts_with("i586") {
        "x86"
    } else {
        "x64"
    };

    if crt_static {
        format!("{arch}-windows-static")
    } else {
        format!("{arch}-windows")
    }
}

fn link_opus_pkg_config() {
    let output = Command::new("pkg-config")
        .args(["--libs", "--cflags", "opus"])
        .output()
        .expect("failed to invoke pkg-config for opus");

    if !output.status.success() {
        panic!("pkg-config failed for opus");
    }

    let flags = String::from_utf8(output.stdout).expect("pkg-config output was not valid UTF-8");
    for token in flags.split_whitespace() {
        if let Some(path) = token.strip_prefix("-L") {
            println!("cargo:rustc-link-search=native={path}");
        } else if let Some(lib) = token.strip_prefix("-l") {
            println!("cargo:rustc-link-lib={lib}");
        }
    }
}

fn build_macos_native() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is missing"));
    let src = PathBuf::from("native/macos_audio.m");
    let obj = out_dir.join("macos_audio.o");
    let lib = out_dir.join("libmacos_audio.a");

    println!("cargo:rerun-if-changed={}", src.display());

    let status = Command::new("clang")
        .args([
            "-fobjc-arc",
            "-c",
            src.to_str().expect("invalid macos_audio.m path"),
            "-o",
            obj.to_str().expect("invalid macos_audio.o path"),
        ])
        .status()
        .expect("failed to run clang for native macOS audio bridge");

    if !status.success() {
        panic!("clang failed to compile native/macos_audio.m");
    }

    let status = Command::new("ar")
        .args([
            "crs",
            lib.to_str().expect("invalid libmacos_audio.a path"),
            obj.to_str().expect("invalid macos_audio.o path"),
        ])
        .status()
        .expect("failed to archive native macOS audio bridge");

    if !status.success() {
        panic!("ar failed to archive native macOS audio bridge");
    }

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=macos_audio");
    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=framework=AVFoundation");
    println!("cargo:rustc-link-lib=framework=AudioToolbox");
    println!("cargo:rustc-link-lib=framework=CoreMedia");
    println!("cargo:rustc-link-lib=framework=CoreAudio");
}
