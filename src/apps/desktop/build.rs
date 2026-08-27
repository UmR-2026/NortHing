fn main() {
    // 2026-08-07: embed an application manifest declaring ComCtl32 v6.
    //
    // `muda` (via the tray-icon stack) calls `TaskDialogIndirect`
    // unconditionally. That symbol is exported
    // only by ComCtl32.dll v6; without a manifest the loader binds v5.82,
    // which lacks it, and the process dies with STATUS_ENTRYPOINT_NOT_FOUND
    // (0xC0000139) before `main` runs — no log output, no panic.
    //
    // rustc embeds a default manifest (trustInfo + compatibility only).
    // Supplying our own replaces it, so northhing.exe.manifest reproduces
    // both of those sections verbatim and adds the v6 dependency.
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=northhing.rc");
        println!("cargo:rerun-if-changed=northhing.exe.manifest");
        embed_windows_manifest();
    }
}

/// Compile `northhing.rc` (which references the application manifest) into a
/// COFF object and hand it to the linker.
///
/// Uses `windres` from the MSYS2 MinGW toolchain, matching the
/// `x86_64-pc-windows-gnu` target this app is built with.
#[cfg(windows)]
fn embed_windows_manifest() {
    use std::path::PathBuf;
    use std::process::Command;

    // Only the GNU toolchain uses windres; MSVC consumes the .manifest
    // directly via a link arg.
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR not set"));

    if env == "msvc" {
        let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("northhing.exe.manifest");
        println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-bins=/MANIFESTINPUT:{}",
            manifest.display()
        );
        return;
    }

    let obj = out_dir.join("northhing_manifest.o");
    let windres = std::env::var("WINDRES").unwrap_or_else(|_| "windres".to_string());

    let status = Command::new(&windres)
        .arg("northhing.rc")
        .arg("-O")
        .arg("coff")
        .arg("-o")
        .arg(&obj)
        .status();

    match status {
        Ok(s) if s.success() => {
            // Pass the object straight to the linker for binaries only, so
            // the resource is not duplicated into test harnesses.
            println!("cargo:rustc-link-arg-bins={}", obj.display());
        }
        Ok(s) => panic!(
            "windres failed with status {s}: cannot embed the ComCtl32 v6 manifest, \
             which is required or the binary dies at load time with 0xC0000139"
        ),
        Err(e) => panic!(
            "failed to run `{windres}` ({e}): it is needed to embed the ComCtl32 v6 \
             manifest, without which the binary dies at load time with 0xC0000139. \
             Ensure the MSYS2 MinGW bin directory (C:\\msys64\\mingw64\\bin) is on PATH."
        ),
    }
}
