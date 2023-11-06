build:
	cargo build

test:
	cargo test

lint:
	cargo +nightly clippy --all-targets -- -D warnings

workspace-optimize:
    #!/bin/bash
    docker run --rm -v "$(pwd)":/code \
        --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
        --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
        --platform linux/amd64 \
        cosmwasm/workspace-optimizer:0.12.13

optimize:
    ./optimize.sh

mv-contracts:
    ls artifacts/

    mkdir -p swap-covenant/tests/interchaintest/wasms
    cp -R artifacts/*.wasm swap-covenant/tests/interchaintest/wasms
    mkdir -p two-party-pol-covenant/tests/interchaintest/wasms
    cp -R artifacts/*.wasm two-party-pol-covenant/tests/interchaintest/wasms
    mkdir -p single-party-pol-covenant/tests/interchaintest/wasms
    cp -R artifacts/*.wasm single-party-pol-covenant/tests/interchaintest/wasms

swap-covenant:
    mkdir -p swap-covenant/tests/interchaintest/wasms
    cp -R artifacts/*.wasm swap-covenant/tests/interchaintest/wasms
    ls swap-covenant/tests/interchaintest/wasms/
    cd swap-covenant/tests/interchaintest && go test --timeout 30m

two-party-pol-covenant:
    mkdir -p two-party-pol-covenant/tests/interchaintest/wasms
    cp -R artifacts/*.wasm two-party-pol-covenant/tests/interchaintest/wasms
    cp -R two-party-pol-covenant/astroport/*.wasm two-party-pol-covenant/tests/interchaintest/wasms
    ls two-party-pol-covenant/tests/interchaintest/wasms/
    cd two-party-pol-covenant/tests/interchaintest && go test --timeout 30m

single-party-pol-covenant:
    mkdir -p single-party-pol-covenant/tests/interchaintest/wasms
    cp -R artifacts/*.wasm single-party-pol-covenant/tests/interchaintest/wasms
    cp -R single-party-pol-covenant/astroport/*.wasm single-party-pol-covenant/tests/interchaintest/wasms
    ls single-party-pol-covenant/tests/interchaintest/wasms/
    cd single-party-pol-covenant/tests/interchaintest && go test --timeout 30m

local-e2e-rebuild TEST: optimize
    #!/usr/bin/env sh
    if [[ $(uname -m) =~ "arm64" ]]; then
        for file in ./artifacts/*-aarch64.wasm; do
            if [ -f "$file" ]; then
                new_name="${file%-aarch64.wasm}.wasm"
                mv "$file" "./$new_name"
            fi
        done
    fi
    cp -R artifacts/*.wasm {{TEST}}/tests/interchaintest/wasms
    ls {{TEST}}/tests/interchaintest/wasms
    cd {{TEST}}/tests/interchaintest/ && go test -timeout 30m -v

local-e2e TEST:
    cd {{TEST}}/tests/interchaintest/ && go test -timeout 30m -v
