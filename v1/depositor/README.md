# Depositor

This module is the depositor in stride-covenant system.
It is responsible for the following tasks:
1. Instantiating an ICA on gaia
1. Transfering Atom from gaia ICA to itself via IBC
1. Splitting the available Atom in half and funding the LP and LS modules

The contract determines the next actions to take purely based on its own state.
After receiving a `tick: {}` message from the clock, it attempts to advance the state.

