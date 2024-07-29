# astroport tokenfactory-based liquid pooler

Contract responsible for providing liquidity to a specified pool.

This is a modification of the [old astroport liquid pooler contract](../astroport-liquid-pooler/README.md)
which works with cw20-based LP tokens.

The only changes between that contract and this one should be isolated
to the way `ExecuteMsg::Withdraw` is handled.
This contract modifies that handler to work with the native tokenfactory
LP tokens. No other changes should be necessary.

## Testing

Because of different astroport versions causing conflicts in the same
workspace, this contract is currently tested on its dedicated
[v0.1.1 release branch](https://github.com/timewave-computer/covenants/tree/release/v0.1.1).

The local-interchaintest directory there contains the existing e2e tests
that are modified to use this contract code id instead of the old one.

To ensure that the tests are working as expected, make sure to confirm the
checksum between the contract that results from wasm optimization in this repo
and the one in `release/v0.1.1` matches.

## Instantiation

The following parameters are expected to instantiate the liquid pooler:

`pool_address` - address of the liquidity pool we wish to interact with

`clock_address` - address of the authorized clock contract to receive ticks from

`slippage_tolerance` - optional parameter to specify the acceptable slippage tolerance for providing liquidity

`assets` - TODO

`single_side_lp_limits` - TODO

`expected_pool_ratio` - the price at which we expect to provide liquidity at

`acceptable_pool_ratio_delta` - the acceptable deviation from the expected price above

`pair_type` - the expected pair type of the pool we wish to enter. used for validation of cases where pool migrates.

## Flow

After instantiation, liquid pooler continuously attempts to provide liquidity to the specified pool.
If possible, double sided liquidity is provided. If it is not, liquid pooler attempts to provide single-sided liquidity.
If neither are possible, nothing happens until the next tick is received, at which point it retries.
