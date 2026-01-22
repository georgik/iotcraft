#!/bin/bash
# Build script for desktop simulator

cd "$(dirname "$0")"

echo "Building IotCraft Desktop Simulator..."
rm -rf build
mkdir build
cd build
cmake ..
make

if [ $? -eq 0 ]; then
    echo ""
    echo "Build successful! Binary: build/iotcraft-desktop"
    echo ""
    echo "Usage examples:"
    echo "  ./build/iotcraft-desktop --verbose"
    echo "  ./build/iotcraft-desktop --screenshot output.png --duration 3"
    echo "  ./build/iotcraft-desktop --chessboard"
else
    echo ""
    echo "Build failed!"
    exit 1
fi
