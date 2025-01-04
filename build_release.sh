#!/bin/bash
# Require that there is one argument ad print usage otherwise
if [ $# -ne 1 ]; then
    echo "Usage: $0 <version>"
    exit 1
fi

VERSION=$1
trunk build --release

# Tar contents of dist/ but not the directory itself
tar -czf "gnc-trainer-$VERSION.tar.gz" -C dist .

# Publish the tarball to the releases page on Github
# Create a new release

# Use `brew install gh`  to install gh
# Assumes that GH_TOKEN is set in the environment
gh release create $VERSION "gnc-trainer-$VERSION.tar.gz" --title "Release $VERSION"
