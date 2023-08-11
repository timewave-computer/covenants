# IBC Deposit Splitter

IBC Deposit Splitter is meant to facilitate receiving funds to an ICA on a remote chain and splitting them up.

To perform the split, a combined `BankSend` is performed to all destination ICAs controlled by forwarder contracts.
This way the transfer is atomic and any of the transfers failing would fail the whole transaction.
Because of no IBC element involved in this step, we can be sure that the transfer succeeded and we may begin
forwarding the funds from the ICAs.

Deposit Splitter should be instantiated with a vector of forwarder modules along with their respective amounts (`Vec<Addr, Uint128>`).
It should then query each IBC Forwarder for their deposit addresses, which are going to be their ICA addresses.

If the addresses are known, and Deposit Splitter ICA has received its funds, it attempts a combined BankSend from its ICA to all the receivers.
