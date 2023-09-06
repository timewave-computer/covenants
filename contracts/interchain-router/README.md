# Interchain Router

Interchain Router is a contract that facilitates a predetermined routing of funds.
Each instance of the interchain router is associated with a single receiver.

The router continuously attempts to perform IBC transfers to the receiver.
In case the IBC transfer fails, the funds will be refunded, and we can safely try again.
