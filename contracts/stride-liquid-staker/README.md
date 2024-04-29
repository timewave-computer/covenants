# Stride Liquid Staker

The Ls creates an Interchain Account on a host chain that temporarily holds funds. It allows anyone to permissionlessly forward funds denominated in a preset token to a preset destination address with an IBC transfer. See `src/msg.rs` for the API.

## Example usecase

The current intended usescase is to create a covenant controlled Interchain Account on Stride. The covenant plans to liquid stake Atom using Stride's Autopilot 1-click liquid stake feature. Stride's Autopilot feature enables IBC transfers to a receiving address on Stride to be automatically liquid staked and also for these liquid staked vouchers to optionally be forwarded over IBC to a destination address. The current use of the contract is to register the receiving address as an ICA on Stride and allow anybody to forward liquid staked Atom from that ICA to the LPer contract. The benefit here is that if Stride's Autopilot IBC forwarding is disabled or otherwise fails, any user can recover the funds by forwarding them to the LPer.

## Flow

First, an ICA needs to be created on Stride.

```md
     contract_state: instantiated

    ┌───────────────────────────────────┐       ┌───────────────────────────┐
    │neutron                            │       │stride                     │
    │                                   │       │                           │
    │ ┌───────────┐                     │       │                           │
    │ │   clock   │                     │       │                           │
    │ └───────────┘                     │       │                           │
    │       │                           │       │                           │
    │       │        ┌────────┐         │       │                           │
    │      tick      │ stride │         │       │          ┌───────┐        │
    │       └───────▶│ liquid │─────1. register_ica───────▶│  ica  │        │
    │                │ staker │         │       │          └───────┘        │
    │                └────────┘         │       │              │            │
    │                     ▲             │       │              │            │
    │                     │             │       │              │            │
    │                     └─────────────┼───────┼──────────────┘            │
    │                       1.1. ContractState::IcaCreated                  │
    │                                   │       │                           │
    └───────────────────────────────────┘       └───────────────────────────┘
```

Once ICA is created and the ICA address is known, anyone can send it funds to
be liquid staked (using stride's autopilot module).

```md
     contract_state: ica_created
    ┌───────────────────────────────────┐       ┌───────────────────────────┐
    │neutron                            │       │stride    .───────.        │
    │                                   │       │         (anything )       │
    │                                   │       │          `───────'        │
    │                                   │       │              │            │
    │                                   │       │        liquid stake       │
    │                                   │       │              │            │
    │                ┌────────┐         │       │              ▼            │
    │                │ stride │         │       │          ┌───────┐        │
    │                │ liquid │         │       │          │  ica  │        │
    │                │ staker │         │       │          └───────┘        │
    │                └────────┘         │       │                           │
    │                                   │       │                           │
    │                                   │       │                           │
    │                                   │       │                           │
    │                                   │       │                           │
    │                                   │       │                           │
    └───────────────────────────────────┘       └───────────────────────────┘
```

After funds are liquid staked and sitting in the ICA, anyone can call the
permisionless `Transfer { amount: Uint128 }` method to initiate the transfer.
The denom and destination is configured on the contract level, so all that
a user can do is initiate that transfer for some amount.

```md
     contract_state: ica_created
    ┌───────────────────────────────────┐       ┌───────────────────────────┐
    │neutron                            │       │stride    .───────.        │
    │                                   │       │         (anything )       │
    │                                   │       │          `───────'        │
    │                                   │       │              │            │
    │                                   │       │      transfer x amount    │
    │                                   │       │              │            │
    │        ┌───────────────┐          │       │              ▼            │
    │        │               │      ibc send user input    ┌───────┐        │
    │        │  destination  │◀───amount to predetermined──│  ica  │        │
    │        │               │          destination        └───────┘        │
    │        └───────────────┘          │       │                           │
    │                                   │       │                           │
    │                                   │       │                           │
    │                                   │       │                           │
    │                                   │       │                           │
    │                                   │       │                           │
    └───────────────────────────────────┘       └───────────────────────────┘
```
