#!/bin/bash

ICON_NAME="icon"
ICONSET_DIR="assets/${ICON_NAME}.iconset"
PNG_FILE="docs/icon.png"
ICNS_FILE="assets/${ICON_NAME}.icns"

# Clean up old iconset if it exists
rm -rf "$ICONSET_DIR"
mkdir "$ICONSET_DIR"

# Generate required sizes
sips -z 16 16     "$PNG_FILE" --out "$ICONSET_DIR/icon_16x16.png"
sips -z 32 32     "$PNG_FILE" --out "$ICONSET_DIR/icon_16x16@2x.png"
sips -z 32 32     "$PNG_FILE" --out "$ICONSET_DIR/icon_32x32.png"
sips -z 64 64     "$PNG_FILE" --out "$ICONSET_DIR/icon_32x32@2x.png"
sips -z 128 128   "$PNG_FILE" --out "$ICONSET_DIR/icon_128x128.png"
sips -z 256 256   "$PNG_FILE" --out "$ICONSET_DIR/icon_128x128@2x.png"
sips -z 256 256   "$PNG_FILE" --out "$ICONSET_DIR/icon_256x256.png"
sips -z 512 512   "$PNG_FILE" --out "$ICONSET_DIR/icon_256x256@2x.png"
sips -z 512 512   "$PNG_FILE" --out "$ICONSET_DIR/icon_512x512.png"
cp "$PNG_FILE" "$ICONSET_DIR/icon_512x512@2x.png"  # original size for retina

# Create .icns
iconutil -c icns "$ICONSET_DIR" -o "$ICNS_FILE"

echo "✅ Generated $ICNS_FILE"
