#!/usr/bin/env bash
# quick-test.sh - Run tests without heavy features

export JCODE_DEV_FEATURE_PROFILE=minimal
echo "Using JCODE_DEV_FEATURE_PROFILE=minimal"

cargo test -p jcode "$@"
