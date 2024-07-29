# astroport liquid pooler

Contract responsible for providing liquidity to a specified pool.

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
