#!/bin/bash
echo "Testing Claudia build..."
echo

# Test egui frontend
echo "Building egui frontend..."
cd src-egui
cargo build
if [ $? -eq 0 ]; then
    echo "✓ Egui frontend built successfully"
else
    echo "✗ Egui frontend build failed"
    exit 1
fi
cd ..

# Test Tauri backend
echo
echo "Building Tauri backend..."
cd src-tauri
cargo build
if [ $? -eq 0 ]; then
    echo "✓ Tauri backend built successfully"
else
    echo "✗ Tauri backend build failed"
    exit 1
fi
cd ..

echo
echo "All builds completed successfully!"