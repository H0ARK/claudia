@echo off
echo Building Claudia Workspace...
echo.

echo Checking Rust installation...
rustc --version
if errorlevel 1 (
    echo ERROR: Rust is not installed or not in PATH
    exit /b 1
)

echo.
echo Building workspace...
cargo build --workspace
if errorlevel 1 (
    echo ERROR: Workspace build failed
    exit /b 1
)

echo.
echo Building Tauri backend...
cd src-tauri
cargo build
if errorlevel 1 (
    echo ERROR: Tauri backend build failed
    exit /b 1
)
cd ..

echo.
echo Building egui frontend...
cd src-egui
cargo build
if errorlevel 1 (
    echo ERROR: Egui frontend build failed
    exit /b 1
)
cd ..

echo.
echo Build completed successfully!