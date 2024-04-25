# astroport liquid pooler

Contract responsible for providing liquidity to a specified pool.

## Instantiation

The following parameters are expected to instantiate the liquid pooler:

`pool_address` - address of the liquidity pool we wish to interact with

`clock_address` - address of the authorized clock contract to receive ticks from

`slippage_tolerance` - optional parameter to specify the acceptable slippage tolerance for providing liquidity

`assets` - denoms we expect to make up the liquidity pool we wish to interact with

`single_side_lp_limits` - absolute units (`Uint128`) that define the max amount
of a denom that can be lp'd single-sided

`expected_pool_ratio` - the price at which we expect to provide liquidity at

`acceptable_pool_ratio_delta` - the acceptable deviation from the expected price above

`pair_type` - the expected pair type of the pool we wish to enter. used for validation of cases where pool migrates.

## Providing liquidity

After instantiation, liquid pooler continuously attempts to provide liquidity to the specified pool.
If possible, double sided liquidity is provided. If it is not, liquid pooler attempts to provide single-sided liquidity.
If neither are possible, nothing happens until the next tick is received, at which point it retries.

```md
    ┌───────────────────────────────────────────────────────────────────────┐
    │ neutron                                                               │
    │                           ┌────────┐                                  │
    │  ┌───────────┐            │ astro  │                                  │
    │  │   clock   │───tick────▶│ liquid │──────1. validate pool ───┐       │
    │  └───────────┘            │ pooler │            price         │       │
    │                           └────────┘                          │       │
    │                                │                              │       │
    │                                │                              │       │
    │                    2. query own balances of                   │       │
    │                           pool denoms                         ▼       │
    │                                │                      ┌──────────────┐│
    │                                │                      │astroport pool││
    │                                ▼                      └──────────────┘│
    │                                x                              ▲       │
    │                   both        ╱ ╲       either                │       │
    │               ┌───pool ──────▕   ▏───────pool ───┐            │       │
    │               │  denoms       ╲ ╱        denom   │            │       │
    │               │                V                 │      2.1. provide  │
    │               │                │                 │        liquidity   │
    │               │             neither              │            │       │
    │               │                │                 │            │       │
    │               ▼                ▼                 ▼            │       │
    │       ┌──────────────┐    ┌─────────┐    ┌───────────────┐    │       │
    │       │double side lp│    │  noop   │    │single side lp │    │       │
    │       └──────────────┘    └─────────┘    └───────────────┘    │       │
    │               │                                  │            │       │
    │               │                                  │            │       │
    │               └──────────────────────────────────┴────────────┘       │
    │                                                                       │
    │                                                                       │
    └───────────────────────────────────────────────────────────────────────┘
```

## Withdrawing liquidity

Authorized holder can initiate a withdraw request. This will attempt to
redeem the LP tokens for any underlying assets.
After underlying assets arrive to the liquid pooler contract, they will
be forwarder to the holder along with a `Distribute` message, which will
conclude the withdraw flow.

```md
    ┌────────────────────────────────────────────────────────────────────┐
    │ neutron                                                            │
    │   ┌───────────┐                                                    │
    │   │  holder   │──1. withdraw ──┐                                   │
    │   └───────────┘   liquidity    │                                   │
    │         ▲                      ▼                                   │
    │         │                 ┌────────┐                               │
    │         │                 │ astro  │                               │
    │      1.3. Distribute msg──│ liquid │────1.1. redeem lp ───┐        │
    │        with withdrawn     │ pooler │        tokens        │        │
    │            tokens         └────────┘                      ▼        │
    │                                ▲                  ┌──────────────┐ │
    │                                └───1.2. return ───│astroport pool│ │
    │                                 underlying assets └──────────────┘ │
    │                                                                    │
    └────────────────────────────────────────────────────────────────────┘
```
