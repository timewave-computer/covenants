# swap covenant

Contract responsible for orchestrating the flow for a tokenswap between
two parties.

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
    ┌────────────────────────────────────────────────────────────────────┐
    │neutron                                                             │
    │ ┌────────┐                                                         │
    │ │  swap  │                                               ┌───────┐ │
    │ │covenant│────────────1.1. init & whitelist─────────────▶│ clock │ │
    │ └────────┘                                               └───────┘ │
    │      │                                                       ▲     │
    │      │                                   ┌─────────────┐     │     │
    │      │                                   │   native    │  enqueue  │
    │      ├─────────────1.4. init────────────▶│  splitter   │─────┤     │
    │      │                                   └─────────────┘     │     │
    │      │                 ┌──────────┐      ┌─────────────┐     │     │
    │      │             1.6. init      │      │ party A ibc │     │     │
    │      │                 │          └─────▶│  forwarder  │─────┤     │
    │      │             ┌──────┐              └─────────────┘     │     │
    │      │     x    ┌─▶│remote│              ┌─────────────┐     │     │
    │      │    ╱ ╲   │  └──────┘              │ party A ic  │     │     │
    │      │   ╱   ╲  │      │          ┌─────▶│   router    │─────┤     │
    │      ├─▶▕party▏─┤  1.2. init      │      └─────────────┘     │     │
    │      │   ╲ A ╱  │      └──────────┘      ┌─────────────┐     │     │
    │      │    ╲ ╱   │  ┌──────┐              │   party A   │     │     │
    │      │     V    └─▶│native│──1.2. init──▶│native router│─────┤     │
    │      │             └──────┘              └─────────────┘     │     │
    │      │                 ┌──────────┐      ┌─────────────┐     │     │
    │      │             1.7. init      │      │ party B ibc │     │     │
    │      │                 │          └─────▶│  forwarder  │─────┤     │
    │      │             ┌──────┐              └─────────────┘     │     │
    │      │     x    ┌─▶│remote│              ┌─────────────┐     │     │
    │      │    ╱ ╲   │  └──────┘              │ party B ic  │     │     │
    │      │   ╱   ╲  │      │          ┌─────▶│   router    │─────┤     │
    │      ├─▶▕party▏─┤  1.3. init      │      └─────────────┘     │     │
    │      │   ╲ B ╱  │      └──────────┘      ┌─────────────┐     │     │
    │      │    ╲ ╱   │  ┌──────┐              │   party B   │     │     │
    │      │     V    └─▶│native│──1.3. init──▶│native router│─────┤     │
    │      │             └──────┘              └─────────────┘     │     │
    │      │                                   ┌─────────────┐     │     │
    │      └────────────1.5. init─────────────▶│ swap holder │─────┘     │
    │                                          └─────────────┘           │
    └────────────────────────────────────────────────────────────────────┘
```
