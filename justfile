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


swap-covenant:
    cd swap-covenant/
    
    mkdir -p tests/interchaintest/wasms

    cp -R ./../artifacts/*.wasm tests/interchaintest/wasms

    go clean -testcache
    cd tests/interchaintest/ && go test -timeout 30m -v ./...
