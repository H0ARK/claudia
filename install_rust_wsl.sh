#!/bin/bash

echo "Installing Rust and Cargo in WSL..."
echo

# Download and install rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o rustup-init.sh
chmod +x rustup-init.sh
./rustup-init.sh -y
rm rustup-init.sh

# Source the cargo environment
source "$HOME/.cargo/env"

# Verify installation
echo
echo "Verifying installation..."
rustc --version
cargo --version

echo
echo "Rust installation complete!"
echo "You may need to run: source ~/.cargo/env"
echo "Or restart your terminal for the PATH to update."