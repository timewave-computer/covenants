# covenant

Contract responsible for orchestrating the LP, LS, and depositor contract flow.

## flow

clock -> holder -> lp -> ls -> depositor

1. instantiate clock, holder
1. instantiate lp with clock and holder addresses
1. instantiate ls with clock and lper addresses
1. tick ls, instantiate ICA
1. instantiate depositor with stride ICA, lper, and clock addresses
1. tick depositor to instantiate gaia ICA
1. tick depositor to LS on stride
1. tick LS to transfer stuatom to LP
1. tick depositor to transfer atom from gaia ICA to LP
1. tick LP to provide liquidity on astroport
1. tick LP to withdraw liquidity
1. tick LP to transfer withdrawn tokens to holder
1. authorized withdrawer withdraws tokens from holder

