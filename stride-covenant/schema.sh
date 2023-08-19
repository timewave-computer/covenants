#!/bin/bash

START_DIR=$(pwd); \
for f in ../contracts/*; do \
    echo "generating schema"; \
    cd "$f"; \
    CMD="cargo run --example schema"; \
    eval ${CMD} > /dev/null; \
    rm -rf ./schema/raw; \
    cd "$START_DIR"; \
done