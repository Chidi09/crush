use std::{env, path::PathBuf, process::Command};

fn main() {
    // Only compile eBPF programs when the feature is enabled and we're on Linux.
    if env::var("CARGO_FEATURE_EBPF").is_err() {
        return;
    }
    if env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() != "linux" {
        return;
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
    let ebpf_crate = workspace_root.join("crates").join("crush-ebpf-progs");

    // Output dir for the compiled ELF
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let elf_out = out_dir.join("crush-ebpf-progs");

    let status = Command::new("cargo")
        .args([
            "+nightly",
            "build",
            "--release",
            "--target",
            "bpfel-unknown-none",
            "-Z",
            "build-std=core",
        ])
        .current_dir(&ebpf_crate)
        .env(
            "CARGO_TARGET_DIR",
            out_dir.join("ebpf-target").to_str().unwrap(),
        )
        .status()
        .expect("failed to execute cargo for eBPF crate");

    assert!(status.success(), "eBPF crate build failed");

    let compiled = out_dir
        .join("ebpf-target")
        .join("bpfel-unknown-none")
        .join("release")
        .join("crush-ebpf-progs");

    std::fs::copy(&compiled, &elf_out).expect("failed to copy eBPF ELF");

    println!("cargo:rustc-env=EBPF_PROG_PATH={}", elf_out.display());
    println!("cargo:rerun-if-changed={}", ebpf_crate.join("src").display());
    println!("cargo:rerun-if-changed={}", ebpf_crate.join("Cargo.toml").display());
}
