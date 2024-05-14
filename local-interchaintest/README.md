# local interchaintest

## setup

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
