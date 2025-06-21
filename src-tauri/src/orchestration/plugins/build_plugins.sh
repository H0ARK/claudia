#!/bin/bash

# Build script for orchestration plugins
# This script compiles plugin source files into dynamic libraries

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
OUTPUT_DIR="$SCRIPT_DIR/../../target/plugins"

# Create output directory if it doesn't exist
mkdir -p "$OUTPUT_DIR"

echo "Building orchestration plugins..."

# Detect OS and set appropriate file extension
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    EXT="so"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    EXT="dylib"
elif [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" || "$OSTYPE" == "win32" ]]; then
    EXT="dll"
else
    echo "Unsupported OS: $OSTYPE"
    exit 1
fi

# Build example plugin
if [ -f "$SCRIPT_DIR/example_plugin.rs" ]; then
    echo "Building example_plugin..."
    rustc --crate-type cdylib \
        -O \
        -o "$OUTPUT_DIR/example_plugin.$EXT" \
        "$SCRIPT_DIR/example_plugin.rs"
    
    if [ $? -eq 0 ]; then
        echo "✓ example_plugin.$EXT built successfully"
    else
        echo "✗ Failed to build example_plugin"
        exit 1
    fi
fi

# You can add more plugins here following the same pattern
# For example:
# if [ -f "$SCRIPT_DIR/another_plugin.rs" ]; then
#     echo "Building another_plugin..."
#     rustc --crate-type cdylib \
#         -O \
#         -o "$OUTPUT_DIR/another_plugin.$EXT" \
#         "$SCRIPT_DIR/another_plugin.rs"
# fi

echo ""
echo "All plugins built successfully!"
echo "Plugins are located in: $OUTPUT_DIR"
echo ""
echo "To use these plugins:"
echo "1. Copy them to your designated plugin directory"
echo "2. Use the PluginRegistry to load them at runtime"