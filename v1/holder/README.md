A holder contract holds LP tokens and funds in the covenant until withdrawn by an authorized `withdrawer` address. API is in src/msg.rs.

The `withdrawer` can:
- `WithdrawLiquidity{}` which results in burning LP tokens held in the holder that are associated with a specified `lp_address` for an Astroport liquidity pool and in exchange redeeming them for the share of liquidity in that pool.
- `Withdraw{}` which results in withdrawing funds held in the holder.