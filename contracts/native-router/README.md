# Native Router

Native Router is a contract that facilitates a predetermined routing of funds.
Each instance of the native router is associated with a single receiver.

The router continuously attempts to perform bank sends to the receiver.
Upon receiving a `Tick`, the contract queries its own balances and uses them
to generate bank transfer messages to the destination address.
