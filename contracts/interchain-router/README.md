# Interchain Router

Interchain Router is a contract that facilitates a predetermined routing of funds.
Each instance of the interchain router is associated with a single receiver.

The router continuously attempts to perform IBC transfers to the receiver.
Upon receiving a `Tick`, the contract queries its own balances and uses them
to generate ibc transfer messages to the destination address.

In case any of the IBC transfers fail, the funds will be refunded, and we can safely try again.

```
       ┌──────────────┐
    ┌──│    clock     │
    │  └──────────────┘
    │
    │  ┌────────────────────────────┐              ┌────────────────────────────┐
    │  │neutron                     │              │remote chain                │
    │  │        ┌─────────────┐     │              │                            │
    │  │        │ interchain  │     │  submit ibc  │        ┌───────┐           │
    └──┼─tick──▶│   router    │─────transfer messages──────▶│address│           │
       │        └─────────────┘     │   (if any)   │        └───────┘           │
       │                            │              │                            │
       │   router queries its own   │              │                            │
       │ balances of preconfigured  │              │                            │
       │target denoms and builds ibc│              │                            │
       │  transfer messages to its  │              │                            │
       │        destination         │              │                            │
       └────────────────────────────┘              └────────────────────────────┘
```
