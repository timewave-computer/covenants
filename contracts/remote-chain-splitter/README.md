# Remote chain splitter

Remote Chain Splitter is a module meant to facilitate predefined splitting of funds on a remote chain.

First, splitter creates an ICA on the specified chain.
Once the ICA address is known, splitter waits for the funds to arrive.

During instantiation, a vector of forwarder modules along with their respective amounts (`Vec<Addr, Uint128>`) are specified.
The forwarder modules are then queried for their deposit addresses, which are going to be their respective ICA addresses.

A combined `BankSend` is then performed to the ICAs on the same remote chain.

Remote chain splitter does not complete. In the future, it will be up to the top level covenant to dequeue it from the clock.
