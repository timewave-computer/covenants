# local interchaintest

## setup

### non-ics stride image setup

Prior to running the interchaintests, a modification of the stride image is needed.
We are using the [v9.2.1 tagged version](https://github.com/Stride-Labs/stride/tree/v9.2.1) image.

In there, we alter the `utils/admins.go` as follows to allow minting tokens from our address in the tests:

```go
var Admins = map[string]bool{
-       "stride1k8c2m5cn322akk5wy8lpt87dd2f4yh9azg7jlh": true, // F5
+       "stride1u20df3trc2c2zdhm8qvh2hdjx9ewh00sv6eyy8": true, // F5
        "stride10d07y265gmmuvt4z0w9aw880jnsr700jefnezl": true, // gov module
}
```

Then we use heighliner by strangelove to build a local docker image, [as described in their documentation](https://github.com/strangelove-ventures/heighliner#example-cosmos-sdk-chain-development-cycle-build-a-local-repository):

```bash
# in the stride directory
heighliner build -c stride --local
```

### install interchaintest

```bash
git clone --depth 1 --branch v8.3.0 https://github.com/strangelove-ventures/interchaintest; cd interchaintest; git switch -c v8.3.0
```

```bash
cd local-interchain
```

```bash
# NOTE: your binary will link back to this location of where you install.
# If you rename the folder or move it, you need to `make install` the binary again.
make install
```

### set up path

cd into this directory (`/covenants/local-interchaintest/`).

for `zsh` users:

```bash
echo 'export ICTEST_HOME="$(pwd)"' >> ~/.zshrc && echo 'export PATH="$PATH:$ICTEST_HOME"' >> ~/.zshrc && source ~/.zshrc
```

for `bash` enjoyers:

```bash
echo 'export ICTEST_HOME="$(pwd)"' >> ~/.bashrc && echo 'export PATH="$PATH:$ICTEST_HOME"' >> ~/.bashrc && source ~/.bashrc
```

verify path:

```bash
echo $ICTEST_HOME #should print out the directory of local interchaintest
```

### spinning up the env

```bash
local-ic start neutron_gaia --api-port 42069
```

### running tests

```bash
cargo run --package local-ictest-e2e --bin local-ictest-e2e
```
