#!/bin/sh
set -eu

# Compile
cargo build

# Build docker container
docker build -t galactic-exchange:test .

# Run tests
cargo test
