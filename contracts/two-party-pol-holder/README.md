# Two party POL holder

Two party POL holder is responsible for aggregating funds and facilitating user
intentions like claim or ragequit.

## Depositing funds to Liquid Pooler

Both parties should deposit their funds to (possibly indirectly) the holder.

After holder asserts the expected balances, it forwards the funds to the Liquid Pooler.
This in turn enters the holder into an `Active` state.

Funds are deposited to liquid pooler iff all expected tokens are available.
No partial transfers.

Deposit stage is subject to a deposit deadline (`Expiration`).
Once the deposit deadline expires, refunds are issued to parties that delivered their parts of the covenant.
This can happen if any of the counterparties do not deliver the funds before the deadline expires, as holder attempts to send all expected funds in a combined `BankSend`.

```md
    ┌─────────┐
    │  clock  │  contract state: instantiated
    └─────────┘
         │
       tick
         │
         ▼
    ┌─────────┐
    │two party│
    │ holder  │──────────────────────┐
    └─────────┘                      ▼
                                     x
                                    ╱ ╲
                                   ╱   ╲
       ┌───────────────┐          ╱     ╲
       │  refund any   │         ╱deposit╲
       │covenant denoms│◀──yes──▕ period  ▏──no─────┐
       │ and complete  │         ╲expired╱          ▼
       └───────────────┘          ╲  ?  ╱           x
                                   ╲   ╱           ╱ ╲
                                    ╲ ╱           ╱   ╲
                                     V           ╱     ╲
                                                ╱parties╲
                                               ▕fulfilled▏
                                                ╲deposit╱
                                                 ╲  ?  ╱
                                                  ╲   ╱
                                                   ╲ ╱
                                                    V      ┌─────────────┐
                          ┌─────────────────┐       │      │send deposits│
                          │                 │       │      │  to liquid  │
                          │  keep waiting   │◀──no──┴─yes─▶│ pooler and  │
                          │                 │              │  activate   │
                          └─────────────────┘              └─────────────┘
```

## Lock Period

A `Lock` duration should be stored to keep track of the covenant duration.

After the `Lock` period expires, both parties are allowed to submit `Claim` messages.
A successful claim results in the claiming party's liquidity portion being withdrawn from the
pool, and forwarding the underlying assets to the respective router module.

```md
    ┌─────────┐
    │  clock  │  contract state: active
    └─────────┘
         │
       tick
         │
         ▼
    ┌─────────┐
    │two party│
    │ holder  │──────────────────────┐
    └─────────┘                      ▼
                                     x
                                    ╱ ╲
                                   ╱   ╲
                                  ╱     ╲
       ┌───────────────┐         ╱lockup ╲         ┌─────────────────┐
       │state = expired│◀──yes──▕ period  ▏──no───▶│  keep waiting   │
       └───────────────┘         ╲expired╱         └─────────────────┘
                                  ╲  ?  ╱
                                   ╲   ╱
                                    ╲ ╱
                                     V
```

## Ragequit

A ragequit functionality should be enabled for both parties that may wish to break their part of the covenant.
Ragequitting party is subject to a percentage based penalty agreed upon instantiation.

Ragequit works differently depending on the type of liquidity agreement.

### Side-based ragequit

Because in side-based liquidity agreement both parties own their own side of denom,
entire position is exited in order to deliver the expected outcome denominations.

Consider party A contributed denom A, and party B contributed denom B.
A ragequit is configured with a 10% penalty.
When party A initiates the ragequit, the following steps happen:

- entire position is withdrawn, holder receives a mix of denom A and B
- 10% of denom A is forfeited to party B
- to party B holder transfers:
  - 100% of available denom B
  - 10% of available denom A (ragequit penalty)
- to party A holder transfers:
  - 90% of available denom A (100% - ragequit penalty)

Holder no longer holds any LP tokens or party denoms, therefore completing.

### Share-based ragequit

In share-based liquidity agreement parties own a share of the LP tokens,
meaning ragequit is a non-finalizing action.

Consider party A contributed denom A, and party B contributed denom B.
A ragequit is configured with a 10% penalty.
When party A initiates the ragequit, the following steps happen:

- 50% - 10% = 40% of the available lp tokens are redeemed, holder receives a mix of denom A and B
- to party A holder transfers:
  - 100% of available denom B
  - 100% of available denom A

Party B's position is left untouched. Party B now owns 100% of the remaining
(60% with respect to the position before the ragequit was initiated) LP tokens.

Party B is free to submit a `Claim` message, which will not be subject to any
penalties, regardless of what lockups may be configured.

## Flow

After instantiation, holder sits in `Instantiated` state and awaits for both parties to deposit funds.

- Once both deposits are received, holder forwards the funds to the next contract and advances the state to `Active`.
- If one of the parties do deposit their part of the funds, but their counterparty does not, refund is initiated. This happens by sending the deposited funds to the respective router which then takes care of the final fund delivery.

`Active` state is a prerequisite for initiating a `Ragequit`. In case of a ragequit, usual covenant flow is broken:

- The initiating party forfeits part of its funds to the other party.
- After withdrawing the ragequitting party funds, holder forwards them to the respective router contract.
- Other party is no longer subject to the notion of expiry date.
  - It is free to submit a `Claim` which will remove the remaining liquidity and send the underlying funds to the interchain-router.

After holder no longer manages any funds, it advances its state to `Complete`.

Any ticks received while holder is `Active` will trigger a check for expiration.

If covenant is expired, holder state is advanced to `Expired`.
Both parties are free to submit `Claim` messages to the holder.
