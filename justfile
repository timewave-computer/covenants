build:
	cargo build

test:
	cargo test

lint:
	cargo clippy --all-targets -- -D warnings

optimize:
    #!/usr/bin/env sh
    ./optimize.sh
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
    cd {{TEST}}/tests/interchaintest/ && go clean -testcache && go test -timeout 50m -v

local-e2e TEST:
    cd {{TEST}}/tests/interchaintest/ && go clean -testcache && go test -timeout 40m -v
