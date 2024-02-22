# local interchaintest

steps:
1. `cp ./chains/chain_setup.json path_to_local_ic_install/chains`
1. `local-ic start chain_setup`
1. `cargo run --package e2e_testing --bin e2e_testing`
