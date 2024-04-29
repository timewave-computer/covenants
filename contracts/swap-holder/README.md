# Swap Holder

Swap Holder is a contract meant to facilitate a tokenswap covenant between two parties.

It holds a list of parties participating in the swap with amount and denom theyre expected to provide.

If holder receives all expected tokens, it forwards them to the splitter module and completes.

After a specified duration, parties that delivered their funds are allowed to withdraw (or are automatically refunded).

## Instantiated

When swap-holder is in `Instantiated` state, it continuously performs two actions:

1. checks for its deposit deadline and advances state to `Expired` if it is due
2. attempts to transfer expected balances to the next contract and `Complete`

```md
    ┌─────────────┐
    │    clock    │
    └─────────────┘
           │
           │    contract state: instantiated
           │                                 x
           │   ┌─────────────┐ 1. lockup    ╱ ╲                ┌─────────────┐
           └──▶│ swap holder │──expired?──▶▕   ▏───────yes────▶│   expired   │
               └─────────────┘              ╲ ╱                └─────────────┘
                                             V
                                             │
                                             │
                                         1.1. received
                                        both deposits?
                                             │
                                             ▼
                                             x            ┌────────────────────┐
                    ┌─────────────┐         ╱ ╲           │ transfer funds to  │
                    │    noop     │◀──no───▕   ▏────yes──▶│  next contract &   │
                    └─────────────┘         ╲ ╱           │      complete      │
                                             V            └────────────────────┘
```

## Expired

When swap-holder is in `Expired` state, it tries to send any funds that it holds to
predetermined addresses. This will usually mean either of the participating parties.
