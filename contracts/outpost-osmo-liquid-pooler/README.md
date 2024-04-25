# osmo liquid pooler outpost

This is a near-stateless outpost contract designed to provide liquidity iff some preconditions are met. The only state being stored is to handle the callback context
to ensure funds do not get stuck on the contract.

Contract has no notion of state, and therefore queries.

It has one execute message, which contains all of the aforementioned conditions:

```rust
pub enum ExecuteMsg {
    ProvideLiquidity {
        /// id of the pool we wish to provide liquidity to
        pool_id: Uint64,
        /// the price which we expect to provide liquidity at
        expected_spot_price: Decimal,
        /// acceptable delta (both ways) of the expected price
        acceptable_price_spread: Decimal,
        /// slippage tolerance
        slippage_tolerance: Decimal,
        /// limits for single-side liquidity provision
        asset_1_single_side_lp_limit: Uint128,
        asset_2_single_side_lp_limit: Uint128,
    },
}
```

## Providing liquidity flow

In this diagram, regardless of the outcome of `1.1.`, all tokens are returned
to the sender after the join pool attempt.

```md
    ┌─────────────────────────────────────────────────────────────────────┐
    │osmosis                              ┌──1.2. return LP tokens─┐      │
    │                                     │     and/or leftover    │      │
    │                                     ▼         denoms         │      │
    │  ┌───────────┐                 ┌─────────┐               ┌───────┐  │
    │  │           │                 │         │               │       │  │
    │  │ polytone  │    1. provide   │  osmo   │   1.1. join   │ gamm  │  │
    │  │   proxy   │─────liquidity──▶│ outpost │──────pool────▶│ pool  │  │
    │  │           │                 │         │               │       │  │
    │  └───────────┘                 └─────────┘               │       │  │
    │        ▲                            │                    └───────┘  │
    │        │       1.3. return all      │                               │
    │        └──────available tokens──────┘                               │
    └─────────────────────────────────────────────────────────────────────┘
```

## Withdrawing liquidity flow

In this diagram, regardless of exit pool message outcome, all tokens are
returned to the sender in the callback.

```md
    ┌─────────────────────────────────────────────────────────────────────┐
    │osmosis                             ┌──────1.2. return ─────┐        │
    │                                    │      underlying       │        │
    │                                    ▼       liquidity       │        │
    │  ┌──────────┐                 ┌────────┐              ┌────────┐    │
    │  │          │                 │        │              │        │    │
    │  │ polytone │   1. withdraw   │  osmo  │  1.1. exit   │  gamm  │    │
    │  │  proxy   │────liquidity───▶│outpost │─────pool────▶│  pool  │    │
    │  │          │                 │        │              │        │    │
    │  └──────────┘                 └────────┘              └────────┘    │
    │        ▲                           │                                │
    │        │       1.3. return all     │                                │
    │        └──────available tokens─────┘                                │
    └─────────────────────────────────────────────────────────────────────┘
```

## Liquidity provision conditions

### pool id

id of the pool we wish to interact with.

### expected spot price and acceptable spread

when submitting a message to provide liquidity from a remote chain, we should
have an idea of what price we wish to provide liquidity at. many things can
happen between submitting a transaction on a remote chain signalling our intent
to provide liquidity, and relayers delivering that message to osmosis.

to circumvent that, we pass our expectations as arguments to this message.
for instance, we could say that for an atom/osmo pool, we expect there to be
10 times as many osmo as there are atom (1:10, or `0.1` in decimal).
prices can fluctuate every block, however, so we also pass an acceptable spread
decimal which we express in absolute terms relative to the actual pool spot price.
meaning, spread could be `0.02`. this will mean that we are fine with joining the
pool if and only if the pool spot price at the time of execution is between `0.08`
and `0.12`.

### slippage tolerance

on top of the acceptable price range, we can also pass a slippage tolerance.
also expressed in decimal, it would be applied by reducing the amount of LP tokens
we would expect to receive from providing liquidity.

after we calculate the expected lp token amount during the execution, we deduct
the slippage tolerance % from that amount, and treat this new number as our target.

### single side lp limits

for both denoms, we pass single-side lp limits. this is an additional layer of safe
guards to avoid providing liquidity at undesirable conditions.
