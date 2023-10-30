# Two party POL holder

## Responsibilities

### Multiple parties

Multiple parties are going to be participating, so the holder should store a list of whitelisted addresses.

### Lock Period

A `Lock` duration should be stored to keep track of the covenant duration.

After the `Lock` period expires, both parties are allowed to submit `Claim` messages.
A successful claim results in the claiming party's liquidity portion being withdrawn from the
pool, and forwarding the underlying assets to the respective router module.

### Ragequit

A ragequit functionality should be enabled for both parties that may wish to break their part of the covenant.
Ragequitting party is subject to a percentage based penalty agreed upon instantiation.

Holder then withdraws the allocation of the ragequitting party (minus the penalty) and forwards the funds to the party.
Counterparty remains in an active position.

Ragequit breaks the regular covenant flow in the following way:

- covenant is no longer subject to expiration
- splitter module no longer gets instantiated, meaning that any pre-agreed upon token distribution split is void
  - both parties receive a 50/50 split of the underlying denoms

### Deposit funds to Liquid Pooler

Both parties should deposit their funds to holder. After holder asserts the expected balances, it forwards
the funds to the Liquid Pooler which then in turn enters into a position.

If party A delivers their part of the covenant deposit agreement but party B fails, party A is refunded.

## Flow

After instantiation, holder sits in `Instantiated` state and awaits for both parties to deposit funds.

- Once both deposits are received, holder forwards the funds to the next contract and advances the state to `Active`.
- If one of the parties do deposit their part of the funds, but their counterparty does not, refund is initiated. This happens by sending the deposited funds to the respective interchain-router which then takes care of the rest.

`Active` state is a prerequisite for initiating a `Ragequit`. In case of a ragequit, usual covenant flow is broken:

- The initiating party forfeits part of its funds to the other party.
- After withdrawing the ragequitting party funds, holder forwards them to the respective interchain-router contract.
- Other party is no longer subject to the notion of expiry date.
  - It is free to submit a `Claim` which will remove the remaining liquidity and send the underlying funds to the interchain-router.

After holder no longer manages any funds, it advances its state to `Complete`.

Any ticks received while holder is `Active` will trigger a check for expiration.

If covenant is expired, holder state is advanced to `Expired`.
Both parties are free to submit `Claim` messages to the holder.
