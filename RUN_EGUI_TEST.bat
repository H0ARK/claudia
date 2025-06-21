@echo off
echo Starting Claudia Orchestrator (egui)...
echo.
echo This will compile and run the egui application.
echo Press Ctrl+C to stop.
echo.

cd src-egui

REM First check if it builds
echo Checking build...
cargo check 2>&1 | findstr /C:"error" >nul
if not errorlevel 1 (
    echo.
    echo BUILD ERRORS DETECTED:
    echo =====================
    cargo check
    echo.
    echo Please share the errors above so they can be fixed.
    pause
    exit /b 1
)

REM If build check passes, run the app
echo Build check passed. Starting application...
echo.
cargo run

if errorlevel 1 (
    echo.
    echo ERROR: Application failed to start
    echo Please check the error messages above
    pause
)