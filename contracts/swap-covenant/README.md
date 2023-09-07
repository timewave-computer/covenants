# swap covenant

Contract responsible for orchestrating flow for a tokenswap between two parties.

## instantiation chain flow

Because of inter-contract dependencies, contracts in the covenant are instantiated in a specific order:
1. clock
1. party A router
1. party B router
1. splitter
1. holder
1. party A forwarder
1. party B forwarder
1. (clock whitelisting)