#!/usr/bin/env bash
# fast-check.sh - Quick cargo check without heavy features
# Use this for fast iteration during development

export JCODE_DEV_FEATURE_PROFILE=minimal
echo "Using JCODE_DEV_FEATURE_PROFILE=minimal (no embeddings, no pdf)"

cargo check -p jcode "$@"
