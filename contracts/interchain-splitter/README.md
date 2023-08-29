# Interchain Splitter

Interchain Splitter is a contract meant to facilitate a pre-agreed upon way of distributing funds at its disposal.

Splitter should remain agnostic to any price changes that may occur during the covenant lifecycle.
It should accept the tokens and distribute them according to the initial agreement.

## Split Configurations

In general, we support a per-denom configuration as follows:
```
OSMO -> [(osmo12323, 40), (cosmos32121, 60)]
ATOM -> [(osmo12323,  50), (cosmo32121, 50)]
USDC -> [timewave_split]
_ -> [(osmo12323, 30), (cosmo32121, 70)]
```

### Timewave Split

Timewave split provides a preconfigured list of addresses.

### Custom Split

A custom split here refers to a list of addresses with their associated share of the split (in %).
In this example `OSMO -> [(osmo12323, 40), (cosmos32121, 60)]`, all OSMO tokens that the splitter
receives will be split between osmo12323 and cosmos32121, with 40% and 60% shares respectively.
Custom split configuration should always add up to 100 or else an error is returned.

### Wildcard Split

For cases where denoms don't really matter, a wildcard split can be provided. Then any denoms that
the splitter holds that do not fall under any of other configurations will be split according to this.

