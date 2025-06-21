@echo off
echo ========================================
echo CLAUDIA BUILD AND VERIFICATION TEST
echo ========================================
echo.

REM Check if cargo is installed
where cargo >nul 2>nul
if errorlevel 1 (
    echo ERROR: Cargo not found. Please install Rust.
    exit /b 1
)

echo [1/4] Checking workspace structure...
cargo check --workspace --message-format short 2>&1
if errorlevel 1 (
    echo ERROR: Workspace check failed
    echo Please run: cargo check --workspace
    exit /b 1
)
echo PASS: Workspace structure OK
echo.

echo [2/4] Building egui frontend...
cd src-egui
cargo build 2>&1 | findstr /C:"error" >nul
if not errorlevel 1 (
    echo ERROR: Egui build has errors
    cargo build
    cd ..
    exit /b 1
)
cargo build >nul 2>&1
if errorlevel 1 (
    echo ERROR: Egui build failed
    cargo build
    cd ..
    exit /b 1
)
echo PASS: Egui frontend builds successfully
cd ..
echo.

echo [3/4] Building Tauri backend...
cd src-tauri
cargo build 2>&1 | findstr /C:"error" >nul
if not errorlevel 1 (
    echo ERROR: Tauri build has errors
    cargo build
    cd ..
    exit /b 1
)
cargo build >nul 2>&1
if errorlevel 1 (
    echo ERROR: Tauri build failed
    cargo build
    cd ..
    exit /b 1
)
echo PASS: Tauri backend builds successfully
cd ..
echo.

echo [4/4] Checking for runtime dependencies...
echo Checking egui dependencies...
cd src-egui
cargo tree -p claudia-egui >nul 2>&1
if errorlevel 1 (
    echo WARNING: Could not verify dependencies
) else (
    echo PASS: Dependencies resolved
)
cd ..

echo.
echo ========================================
echo BUILD VERIFICATION COMPLETE
echo ========================================
echo.
echo Next steps:
echo 1. Run the egui app: cd src-egui && cargo run
echo 2. Run the Tauri app: cd src-tauri && cargo tauri dev
echo.
echo If you see any errors above, please share them so I can fix them.