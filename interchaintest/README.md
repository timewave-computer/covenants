# covenant interchaintest

Covenant e2e testing is done using the interchaintest framework.

`utils/` contains various functions and types for bootstrapping our testing environment.

`swap/` and `two-party-pol/` contains the actual e2e tests.

Tests can be ran by using our `just` helpers. There are two main recipes:

### `local-e2e-rebuild TEST PATTERN='.*': optimize`

This recipe rebuilds the contracts using `wasm-optimizer`.
Resulting wasm files are then copied over into our test directory and tests are ran.

Pattern is an optional parameter for running a specific test.
E.g. to run the two party pol test involving a native and interchain party, run the following:
```sh
just local-e2e-rebuild two-party-pol TestTwoPartyNativePartyPol
```

### `local-e2e TEST PATTERN='.*':`

This recipe does not rebuild the contracts. Instead, existing wasm files found under `artifacts/` directory are used.
This can be used in cases where you only changed the interchaintest code so that you do not need to wait for contracts to be rebuilt and optimized.

Pattern is an optional parameter for running a specific test.
E.g. to run the two party pol test involving a native and interchain party, run the following:
```sh
just local-e2e two-party-pol TestTwoPartyNativePartyPol
```
