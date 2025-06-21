#!/bin/bash

echo "Setting up dependencies for Claudia in WSL..."
echo

# Update package list
echo "Updating package list..."
sudo apt-get update

# Install required dependencies
echo "Installing required packages..."
sudo apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    libwebkit2gtk-4.0-dev \
    libwebkit2gtk-4.1-dev \
    curl \
    wget

echo
echo "Dependencies installed successfully!"
echo

# Source cargo if it exists
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
    echo "Cargo environment loaded."
    cargo --version
fi

echo
echo "Now you can build the project with:"
echo "  cd src-egui && cargo build"
echo "  cd src-tauri && cargo build"