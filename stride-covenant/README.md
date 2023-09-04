# Stride LP covenant

The Stride LP Covenant is a trust minimized tool to allow ATOM holders to enter into an ATOM/stATOM liquidity pool with the passage of a single governance proposal on the Cosmos Hub. 

The tool includes a state machine that performs the following actions sequentially:
* Receive ATOM from Cosmos Hub community pool
* Split received ATOM into two portions 
* Liquid stake first portion of ATOM in return for stATOM
* With the remaining ATOM, join an ATOM/stATOM liquidity pool on Astroport, a decentralized exchange on Neutron

The tool includes mechanisms to:
* Allow whitelisted actors to withdraw liquidity
* Handle failures gracefully, including any failed IBC transactions on remote chains, either automatically or with the aid of permissionless intervention to advance the state machine
* Upgrade contracts with an admin account

## Architecture

The Stride LP covenant uses the following contracts:

1. [Stride Covenant](../contracts/covenant): Instantiates the modules and clock. Holds shared state between contracts.
2. [Depositor](../contracts/depositor/): Creates an ICA account on Cosmos Hub and tries to withdraw ATOM. It then IBC transfers tokens to the subsequent modules (stride with autopilot memo and liquidity pooler contracts)
3. [Liquid Staker](../contracts/ls/): Creates an ICA account on Stride. It allows anyone to permissionlessly forward stATOM on the Stride ICA to be IBC transferred to the Liquidity Pooler.
4. [Liquidity Pooler](../contracts/lper/): provides liquidity to the stATOM/ATOM pool on the Astroport DEX on Neutron. It sends the LP tokens to the Holder module.
5. [Holder](../contracts/holder/): Holds LP tokens. A whitelisted withdrawer can redeem the LP tokens for funds. This whitelisted withdrawer can also withdraw the redeemed funds.
6. [Clock](../contracts/clock/): Send ticks periodically to any of the modules that have requested them to advance the state machine. The clock can itself be invoked by an off chain actor, such as a cron job

![](stride-contracts-overview.png)

## Testing and Audits
Stride LP covenant has been thoroughly tested through
* Unit tests
* E2E tests using [Interchain Test](tests/interchaintest/ics_test.go)
* Mainnet testing on Neutron ([instantiated mainnet contract address](https://neutron.celat.one/neutron-1/contracts/neutron1h9ysm943hnyhhvqglemjcx8tr4j5wkc6q3d6r0vqkg9004rujctsd60zqh))

Informal Systems successfully completed an [independent audit](./17-08-2023-informal-timewave-covenants-audit.pdf) in August 2023.