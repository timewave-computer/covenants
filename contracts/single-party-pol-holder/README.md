# single party POL holder

A single party POL holder exists to facilitate the liquidity unwinding from the liquid pooler contract.

By calling `claim`, covenant party can initiate the withdrawal process.
This will in turn send a `withdraw` message to the liquid pooler for 100% of the available lp tokens.
Once the liquid pooler unwinds the lp tokens, it submits back a `distribute` message with the funds
that had been withdrawn. With that, funds are sent to the `withdraw_to` address which is usually
the router contract responsible for routing the funds to their final destination.
