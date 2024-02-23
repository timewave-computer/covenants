# local interchaintest

steps:
1. `cp ./chains/chain_setup.json path_to_local_ic_install/chains`
1. `local-ic start chain_setup --relayer-uidgid "1000:1000" --relayer-version "v2.4.0" --relayer-startup-flags "-p events --black-history 100 -d --log-format console"`
1. `cargo run --package e2e_testing --bin e2e_testing`
