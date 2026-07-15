$env:Path = "C:\msys64\mingw64\bin;" + $env:Path
cargo llvm-cov --html --output-dir coverage --workspace --exclude-files 'target/*' 'tests/*'
