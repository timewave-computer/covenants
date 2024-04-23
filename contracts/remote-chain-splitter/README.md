# Remote chain splitter

Remote Chain Splitter is a module meant to facilitate predefined splitting of funds on a remote chain.

First, splitter creates an ICA on the specified chain.
Once the ICA address is known, splitter waits for the funds to arrive.

During instantiation, a vector of forwarder modules along with their respective amounts (`Vec<Addr, Uint128>`) are specified.
The forwarder modules are then queried for their deposit addresses, which are going to be their respective ICA addresses.

A combined `BankSend` is then performed to the ICAs on the same remote chain.

Remote chain splitter does not complete. In the future, it will be up to the top level covenant to dequeue it from the clock.

The reason for this design comes from the fact that we cannot reliably perform
fund splitting over ibc. Consider a situation where we wish to split some amount
of denom X on chain A in half - 1/2 of X to be sent to chain B, and the other 1/2
to be sent to chain C. Lets say we submit those ibc transfer messages, and chain
B receives the funds successfully, but ibc message to chain C times out and we
get the refund of 1/2 X. If we were to perform the split again, we would be sending
1/4 of X to chain B and 1/4 of X to chain C, which was not the original intention.

To mitigate such situations, we split the funds on the remote chain before we
start dealing with ibc. `MsgMultiSend` atomically splits the funds into dedicated
interchain accounts we created on that remote chain, giving us the ability to
safely retry any failed ibc transfers in isolation.

```md
       ┌──────────────┐
    ┌──│    clock     │
    │  └──────────────┘
    │  t = 1
    │  ┌────────────────────────────┐            ┌───────────────────────────┐
    │  │neutron                     │            │remote chain               │
    │  │                            │            │                           │
    │  │                            │            │                           │
    │  │         ┌─────────────┐    │            │                           │
    │  │         │remote chain │    │            │ ┌───────┐    2. deposit   │
    ├──┼─tick───▶│  splitter   │──1. register_ica┼▶│  ica  │◀─────funds      │
    │  │         └─────────────┘    │            │ └───────┘        │        │
    │  │                ▲           │            │     │      .───────────.  │
    │  │                │1.1. ContractState::IcaCreated│     (  anything   ) │
    │  │                └───────────┼────────────┼─────┘      `───────────'  │
    │  │                            │            │                           │
    │  └────────────────────────────┘            └───────────────────────────┘
    │
    │
    │    t = 2
    │  ┌────────────────────────────┐            ┌───────────────────────────┐
    │  │neutron                     │            │remote chain               │
    │  │               ┌──────────┐ │            │               ┌─────────┐ │
    │  │               │ next_c 1 │─┼──create────┼──────────────▶│  ica 1  │ │
    │  │               └──────────┘ │            │               └─────────┘ │
    │  │                            │            │                           │
    │  │               ┌──────────┐ │            │               ┌─────────┐ │
    │  │               │ next_c 2 │─┼──create────┼──────────────▶│  ica 2  │ │
    │  │               └──────────┘ │            │               └─────────┘ │
    │  │     ┌─────────────┐        │            │       ┌───────┐           │
    │  │     │remote chain │        │            │       │  ica  │           │
    ├──tick─▶│  splitter   │        │            │       └───────┘           │
    │  │     └─────────────┘        │            │                           │
    │  └────────────────────────────┘            └───────────────────────────┘
    │
    │
    │   t = 3
    │  ┌────────────────────────────┐
    │  │neutron        ┌──────────┐ │            ┌───────────────────────────┐
    │  │           ┌──▶│ next_c 1 │ │            │remote chain               │
    │  │           │   └──────────┘ │            │                           │
    │  │           │   ┌──────────┐ │            │                ┌─────────┐│
    │  │           ├──▶│ next_c 2 │ │            │             ┌─▶│  ica 1  ││
    │  │           │   └──────────┘ │            │             │  └─────────┘│
    │  │1. query deposit            │            │  ┌───────┐  │             │
    │  │     address                │            │  │  ica  │──┤MsgMultiSend │
    │  │           │                │            │  └───────┘  │             │
    │  │    ┌─────────────┐         │            │      ▲      │  ┌─────────┐│
    │  │    │remote chain │         │ 1.1. try   │      │      └─▶│  ica 2  ││
    └─tick─▶│  splitter   │─────────┼split funds─┼──────┘         └─────────┘│
       │    └─────────────┘         │            └───────────────────────────┘
       └────────────────────────────┘
```
