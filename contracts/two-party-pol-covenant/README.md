# two party POL covenant

Contract responsible for orchestrating the flow for a two party POL.

This contract composes the instantiation of all contracts necessary
to fulfill the covenant.

The clock is instantiated first. All necessary contracts that are to be
ticked are enqueued to the clock.

Then we continue with the instantiation of each contract, which enqueue
themselves to the clock as part of their instantiation.

## instantiation flow

Because of inter-contract dependencies, contracts in the covenant are
instantiated in a specific order:

```md
    ┌───────────────────────────────────────────────────────────────────┐
    │neutron                                                            │
    │  ┌────────┐                                                       │
    │  │  two   │                1.1. init &                  ┌───────┐ │
    │  │ party  │─────────────────whitelist──────────────────▶│ clock │ │
    │  │covenant│                                             └───────┘ │
    │  └────────┘                                                 ▲     │
    │       │                                    ┌─────────────┐  │     │
    │       │                                    │  two party  │  │     │
    │       ├────────────────────────1.2. init──▶│   holder    │─enqueue│
    │       │                                    └─────────────┘  │     │
    │       │                  ┌──────────┐      ┌─────────────┐  │     │
    │       │              1.6. init      │      │ party A ibc │  │     │
    │       │                  │          └─────▶│  forwarder  │─enqueue│
    │       │              ┌──────┐              └─────────────┘  │     │
    │       │      x    ┌─▶│remote│              ┌─────────────┐  │     │
    │       │     ╱ ╲   │  └──────┘              │ party A ic  │  │     │
    │       │    ╱   ╲  │      │          ┌─────▶│   router    │─enqueue│
    │       ├──▶▕party▏─┤  1.3. init      │      └─────────────┘  │     │
    │       │    ╲ A ╱  │      └──────────┘      ┌─────────────┐  │     │
    │       │     ╲ ╱   │  ┌──────┐              │   party A   │  │     │
    │       │      V    └─▶│native│──1.3. init──▶│native router│─enqueue│
    │       │              └──────┘              └─────────────┘  │     │
    │       │                  ┌──────────┐      ┌─────────────┐  │     │
    │       │              1.7. init      │      │ party B ibc │  │     │
    │       │                  │          └─────▶│  forwarder  │─enqueue│
    │       │              ┌──────┐              └─────────────┘  │     │
    │       │      x    ┌─▶│remote│              ┌─────────────┐  │     │
    │       │     ╱ ╲   │  └──────┘              │ party B ic  │  │     │
    │       │    ╱   ╲  │      │          ┌─────▶│   router    │─enqueue│
    │       ├──▶▕party▏─┤  1.4. init      │      └─────────────┘  │     │
    │       │    ╲ B ╱  │      └──────────┘      ┌─────────────┐  │     │
    │       │     ╲ ╱   │  ┌──────┐              │   party B   │  │     │
    │       │      V    └─▶│native│──1.4. init──▶│native router│─enqueue│
    │       │              └──────┘              └─────────────┘  │     │
    │       │                                    ┌─────────────┐  │     │
    │       └────────────────────────1.5. init──▶│liquid pooler│─enqueue│
    │                                            └─────────────┘        │
    │                                                                   │
    └───────────────────────────────────────────────────────────────────┘
```
