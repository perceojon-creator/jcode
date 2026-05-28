#!/usr/bin/env bash
# full-build.sh - Build with all features (embeddings + pdf)

export JCODE_DEV_FEATURE_PROFILE=full
echo "Using JCODE_DEV_FEATURE_PROFILE=full (embeddings + pdf)"

cargo build --release -p jcode "$@"
