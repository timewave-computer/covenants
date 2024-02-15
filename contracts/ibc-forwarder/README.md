# IBC Forwarder
```
       ┌──────────────┐
    ┌──│    clock     │
    │  └──────────────┘
    │  t = 1
    │  ┌────────────────────────────┐              ┌────────────────────────────┐
    │  │neutron                     │              │remote chain                │
    │  │         ┌─────────────┐    │              │                            │
    │  │         │next contract│    │              │                            │
    │  │         └─────────────┘    │              │                            │
    │  │                            │              │                            │
    │  │         ┌─────────────┐    │              │                            │
    │  │         │             │    │              │      ┌───────┐             │
    ├──┼──tick──▶│ibc forwarder│───1. register_ica─┼─────▶│  ica  │             │
    │  │         │             │    │              │      └───────┘             │
    │  │         └─────────────┘    │              │          │                 │
    │  │                ▲           │              │          │                 │
    │  └────────────────┼───────────┘              └──────────┼─────────────────┘
    │                   └──1.1. ContractState::IcaCreated─────┘
    │
    │  t = 2
    │  ┌────────────────────────────┐              ┌────────────────────────────┐
    │  │neutron  ┌─────────────┐    │              │remote chain                │
    │  │         │next contract│◀───┼──────────┐   │                            │
    │  │         └──────▲──────┘    │          │   │            ┌───────┐       │
    │  │                │           │       2.3. MsgTransfer────│  ica  │       │
    │  │       2.1. query deposit   │              │            └───────┘       │
    │  │         address & memo     │              │                ▲           │
    │  │                │           │              │                │           │
    │  │         ┌──────┴──────┐    │              │                │           │
    │  │         │             │    │              │                │           │
    └──┼──tick──▶│ibc forwarder│────┼──2.2. forward_funds───────────┘           │
       │         │             │    │              │                            │
       │         └─────────────┘    │              │                            │
       └────────────────────────────┘              └────────────────────────────┘
```

IBC Forwarders are contracts instantiated on neutron with the sole responsibility of
receiving funds to an ICA on a remote chain and forwarding them to another module.

In addition to being aware of all IBC related information such as ICA & IBC transfer
timeouts and channel/connection-ids, forwarder needs to have a destination contract
address.

The destination contract is used to perform a `DepositAddress {}` query, which will return an
`Option<Addr>`. This gives us two cases:

1. `None`, in which case IBC Forwarder does nothing and keeps waiting
1. `Addr`, which is then used as a destination address to forward the funds to

While IBC Forwarder should remain agnostic to any underlying details of what the
deposit address is, a few examples of it may be:

- another ICA address or an autopilot receiver string in case of Liquid Staker
- contract address itself in case of Liquid Pooler

IBC Forwarder needs to receive funds in order to be able to forward them. To enable
that, we expose a `DepositAddress {}` query method. After instantiating its ICA,
forwarder can return that ICA address as its deposit address. Prior to ICA
instantiation the query should be returning `None`, indicating that it is not yet
ready to receive funds.
