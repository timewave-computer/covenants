# local interchaintest

steps:
1. `cp ./chains/neutron_gaia.json path_to_local_ic_install/chains`
1. `local-ic start neutron_gaia --relayer-uidgid "1000:1000" --relayer-version "v2.4.0" --relayer-startup-flags "-p events --black-history 100 -d --log-format console"`
1. `cargo run --package e2e_testing --bin e2e_testing`


VSC validator creation command:
```sh
gaiad tx staking create-validator --amount 1000000uatom --pubkey '{"@type":"/cosmos.crypto.ed25519.PubKey","key":"qwrYHaJ7sNHfYBR1nzDr851+wT4ed6p8BbwTeVhaHoA="}' --moniker a --commission-rate 0.1 --commission-max-rate 0.2 --commission-max-change-rate 0.01 --node tcp://0.0.0.0:26657 --home /var/cosmos-chain/localcosmos-1 --chain-id localcosmos-1 --from faucet --fees 20000uatom --keyring-backend test -y
```
