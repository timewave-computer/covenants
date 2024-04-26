build:
	cargo build

test:
	cargo test

lint:
	cargo clippy --all-targets -- -D warnings

schema:
  #!/usr/bin/env sh
  ./schemagen.sh

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

local-e2e-rebuild TEST PATTERN='.*': optimize
  mkdir -p interchaintest/{{TEST}}/wasms
  cp -R interchaintest/wasms/polytone/*.wasm interchaintest/{{TEST}}/wasms
  cp -R interchaintest/wasms/astroport/*.wasm interchaintest/{{TEST}}/wasms
  cp -R artifacts/*.wasm interchaintest/{{TEST}}/wasms
  ls interchaintest/{{TEST}}/wasms
  cd interchaintest/{{TEST}} && go clean -testcache && go test -timeout 60m -v -run '{{PATTERN}}'

local-e2e TEST PATTERN='.*':
  mkdir -p interchaintest/{{TEST}}/wasms
  cp -R interchaintest/wasms/polytone/*.wasm interchaintest/{{TEST}}/wasms
  cp -R interchaintest/wasms/astroport/*.wasm interchaintest/{{TEST}}/wasms
  cp -R artifacts/*.wasm interchaintest/{{TEST}}/wasms
  cd interchaintest/{{TEST}} && go clean -testcache && go test -timeout 60m -v -run '{{PATTERN}}'
