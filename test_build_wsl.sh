#!/bin/bash

# Source cargo environment
source "$HOME/.cargo/env" 2>/dev/null || {
    echo "Error: Cargo not found. Please install Rust first."
    exit 1
}

echo "========================================="
echo "Testing Claudia Build in WSL"
echo "========================================="
echo

# Change to project directory
cd /mnt/c/Users/conra/OneDrive/Documents/Github/claudia

echo "[1/3] Checking egui frontend..."
cd src-egui
if cargo check 2>&1 | grep -q "error\[E"; then
    echo "❌ FAIL: Egui frontend has compilation errors"
    echo
    cargo check
else
    echo "✅ PASS: Egui frontend compiles successfully"
fi
cd ..

echo
echo "[2/3] Checking Tauri backend..."
cd src-tauri
if cargo check 2>&1 | grep -q "error\[E"; then
    echo "❌ FAIL: Tauri backend has compilation errors"
    echo
    cargo check
else
    echo "✅ PASS: Tauri backend compiles successfully"
fi
cd ..

echo
echo "[3/3] Running egui application..."
cd src-egui
echo "Starting egui orchestrator..."
echo "Press Ctrl+C to stop"
cargo run