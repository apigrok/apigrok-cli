#!/bin/bash
set -e

echo "Building APIGrok..."
cargo build --release

echo ""
echo "Build complete! Binary location:"
echo "  $(pwd)/target/release/apigrok"
echo ""
echo "To install globally, run:"
echo "  cargo install --path ."
echo ""
echo "Or add to your PATH:"
echo "  export PATH=\"\$PATH:$(pwd)/target/release\""
echo ""
echo "Test the binary:"
echo "  ./target/release/apigrok https://httpbin.org/get"
