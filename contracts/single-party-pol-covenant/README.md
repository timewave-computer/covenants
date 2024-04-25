# single party POL covenant

Contract responsible for orchestrating the flow for a two party POL.

This contract composes the instantiation of all contracts necessary
to fulfill the covenant.

The clock is instantiated first. All necessary contracts that are to be
ticked are enqueued to the clock.

Then we continue with the instantiation of each contract, which enqueue
themselves to the clock as part of their instantiation.

```md
    ┌────────────────────────────────────────────────────────────────────┐
    │neutron                                                             │
    │  ┌────────┐                                                        │
    │  │ single │                                             ┌───────┐  │
    │  │ party  │───────────1.1. init & whitelist────────────▶│ clock │  │
    │  │covenant│                                             └───────┘  │
    │  └────────┘                                                 ▲      │
    │       │                                                     │      │
    │       │                     ┌─────────────┐                 │      │
    │       ├─────1.2. init──────▶│liquid staker│─────enqueue─────┤      │
    │       │                     └─────────────┘                 │      │
    │       │                     ┌─────────────┐                 │      │
    │       │                     │single party │                 │      │
    │       ├─────1.3. init──────▶│   holder    │                 │      │
    │       │                     └─────────────┘                 │      │
    │       │                     ┌─────────────┐                 │      │
    │       ├─────1.4. init──────▶│liquid pooler│─────enqueue─────┤      │
    │       │                     └─────────────┘                 │      │
    │       │                     ┌─────────────┐                 │      │
    │       │                     │remote chain │                 │      │
    │       ├─────1.5. init──────▶│  splitter   │─────enqueue─────┤      │
    │       │                     └─────────────┘                 │      │
    │       │                     ┌─────────────┐                 │      │
    │       │                     │ interchain  │                 │      │
    │       ├─────1.6. init──────▶│   router    │─────enqueue─────┤      │
    │       │                     └─────────────┘                 │      │
    │       │                     ┌─────────────┐                 │      │
    │       │                     │liquid staker│                 │      │
    │       ├─────1.7. init──────▶│ibc forwarder│─────enqueue─────┤      │
    │       │                     └─────────────┘                 │      │
    │       │                     ┌─────────────┐                 │      │
    │       │                     │liquid pooler│                 │      │
    │       └─────1.8. init──────▶│ibc forwarder│─────enqueue─────┘      │
    │                             └─────────────┘                        │
    │                                                                    │
    └────────────────────────────────────────────────────────────────────┘
```
