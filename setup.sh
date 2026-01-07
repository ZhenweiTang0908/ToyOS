#!/bin/bash

# NewTownOS Environment Setup Script for macOS

set -e

if ! command -v rustup &> /dev/null; then
    brew install rustup
    rustup-init -y
else
    echo "Rustup installed."
fi

rustup default nightly

rustup component add rust-src
rustup component add llvm-tools-preview

if ! command -v qemu-system-x86_64 &> /dev/null; then
    brew install qemu
else
    echo "QEMU installed."
fi

qemu-system-x86_64 --version

if ! command -v bootimage &> /dev/null; then
    cargo install bootimage
else
    echo "bootimage installed."
fi

echo "Run 'cargo run' to start the OS."
