use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    link_opus();

    let target = env::var("TARGET").unwrap_or_default();
    if target.contains("apple-darwin") {
        build_macos_native();
    }
}

fn link_opus() {
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
