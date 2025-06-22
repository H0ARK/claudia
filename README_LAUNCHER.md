# Claudia Orchestrator Launcher

## Quick Start

Simply run from the root directory:

```bash
cargo run
```

This will start both:
- 📱 **eGUI Frontend** - The user interface for managing workers and orchestrations
- ⚡ **Tauri Backend** - The API server and worker management system

## What Happens

1. The launcher starts the eGUI frontend first
2. After a 2-second delay, it starts the Tauri backend
3. Both applications run concurrently
4. Output from both is displayed in your terminal
5. Press **Ctrl+C** to stop both applications gracefully

## Alternative Commands

If you prefer to run them separately:

```bash
# Run eGUI frontend only
cargo run --manifest-path src-egui/Cargo.toml

# Run Tauri backend only  
cargo run --manifest-path src-tauri/Cargo.toml
```

## Features

- ✅ **Single Command**: Just `cargo run` to start everything
- ✅ **Real-time Output**: See logs from both applications
- ✅ **Graceful Shutdown**: Ctrl+C stops both cleanly
- ✅ **Error Handling**: Clear error messages if something fails
- ✅ **Process Management**: Proper PID tracking and cleanup

## Architecture

```
claudia/
├── src/main.rs           # 🚀 Launcher (starts both)
├── src-egui/             # 📱 eGUI Frontend
└── src-tauri/            # ⚡ Tauri Backend
```

The launcher orchestrates both applications, giving you a seamless development experience. 