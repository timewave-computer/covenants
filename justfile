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

mv-contracts:
    ls artifacts/

    mkdir -p swap-covenant/tests/interchaintest/wasms
    cp -R artifacts/*.wasm swap-covenant/tests/interchaintest/wasms
    mkdir -p two-party-pol-covenant/tests/interchaintest/wasms
    cp -R artifacts/*.wasm two-party-pol-covenant/tests/interchaintest/wasms

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
