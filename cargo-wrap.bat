@echo off
REM cargo-wrap.bat — cmd.exe fallback for the MSYS2/MinGW build environment.
REM
REM Context (2026-06-25): the project compiles against the GNU toolchain
REM (`rustup default stable-x86_64-pc-windows-gnu`) and depends on MinGW
REM tooling (gcc.exe, dlltool.exe) for native C/C++ deps (aws-lc-sys,
REM libgit2-sys). The MSYS2 install at C:\msys64 provides them.
REM
REM This wrapper is now a FALLBACK. The primary path-resolution chain is:
REM   1. User PATH (persistent for all new shells): C:\msys64\mingw64\bin
REM      + C:\msys64\usr\bin, set via [Environment]::SetEnvironmentVariable
REM   2. PowerShell profile (Microsoft.PowerShell_profile.ps1) prepends the
REM      same dirs at every PS session start
REM   3. Git Bash ~/.bashrc already exports the same dirs
REM
REM Use this wrapper ONLY when running cargo from a context where the above
REM env didn't take effect (e.g. an old cmd window opened before the User
REM PATH edit, or a CI runner that strips the env). For normal use just run
REM `cargo` directly.
REM
REM See `.cargo/config.toml` for the project-level linker setting (defense-
REM in-depth: cargo can find gcc via absolute path even if PATH is stripped).

set PATH=C:\msys64\mingw64\bin;C:\msys64\usr\bin;%PATH%
cargo %*