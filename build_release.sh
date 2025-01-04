#!/bin/bash
# Require that there is one argument ad print usage otherwise
if [ $# -ne 1 ]; then
    echo "Usage: $0 <version>"
    exit 1
fi

VERSION=$1
trunk build --release

# Install with `cargo binstall wasm-opt`
FILENAME=$(ls ./dist/*.wasm | head -n1)
wasm-opt -O -ol 100 -s 100 $FILENAME -o $FILENAME

# Tar contents of dist/ but not the directory itself
tar -czf "gnc-trainer-$VERSION.tar.gz" -C dist .
