build:
	cargo build

test:
	cargo test

lint:
	cargo clippy --all-targets -- -D warnings

optimize:
    ./optimize.sh

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

two-party-pol-covenant-native:
    mkdir -p two-party-pol-covenant/tests/interchaintest/wasms
    cp -R artifacts/*.wasm two-party-pol-covenant/tests/interchaintest/wasms
    cp -R two-party-pol-covenant/astroport/*.wasm two-party-pol-covenant/tests/interchaintest/wasms
    ls two-party-pol-covenant/tests/interchaintest/wasms/
    cd two-party-pol-covenant/tests/interchaintest && go clean -testcache && go test --timeout 50m -v -run TestTwoPartyNativePartyPol

local-e2e-rebuild TEST: optimize
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

local-e2e-rebuild TEST PATTERN='.*': optimize
    mkdir interchaintest/{{TEST}}/wasms
    cp -R artifacts/*.wasm interchaintest/{{TEST}}/wasms
    ls interchaintest/{{TEST}}/wasms
    cd interchaintest/{{TEST}} && go clean -testcache && go test -timeout 50m -v -run '{{PATTERN}}'

local-e2e TEST PATTERN='.*':
    cd interchaintest/{{TEST}} && go clean -testcache && go test -timeout 50m -v -run '{{PATTERN}}'
