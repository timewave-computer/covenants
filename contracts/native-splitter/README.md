# Interchain Splitter

Interchain Splitter is a contract meant to facilitate a pre-agreed upon way of distributing funds at its disposal.

Splitter should remain agnostic to any price changes that may occur during the covenant lifecycle.
It should accept the tokens and distribute them according to the initial agreement.

```md
       ┌──────────────┐
    ┌──│    clock     │
    │  └──────────────┘
    │
    │   ┌─────────────────────────────────────────────────────┐
    │   │neutron                                              │
    │   │                                      ┌─────────┐    │
    │   │                          ┌───6atom──▶│cosmos123│    │
    │   │     ┌────────┐           │           └─────────┘    │
    │   │     │ native │   split   │                          │
    └──tick──▶│splitter│──10atom───┤                          │
        │     └────────┘           │                          │
        │                          │           ┌─────────┐    │
        │                          └───4atom──▶│cosmos321│    │
        │                                      └─────────┘    │
        │                                                     │
        └─────────────────────────────────────────────────────┘
```

## Split Configurations

In general, we support a per-denom configuration as follows:

```md
OSMO -> [(cosmos123, 40), (cosmos321, 60)]
ATOM -> [(cosmos123,  50), (cosmos321, 50)]
_ -> [(cosmos123, 30), (cosmos321, 70)]
```

### Custom Split

A custom split here refers to a list of addresses with their associated share of the split (in %).
In this example `OSMO -> [(cosmos123, 40), (cosmos321, 60)]`, all OSMO tokens that
the splitter receives will be split between cosmos123 and cosmos321, with 40% and
60% shares respectively.
Custom split configuration should always add up to 100 or else an error is returned.

### Wildcard Split

For cases where denoms don't really matter, a wildcard split can be provided.
Then any denoms that the splitter holds that do not fall under any of other configurations will be split according to this.

