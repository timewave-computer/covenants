# Swap Holder

Swap Holder is a contract meant to facilitate a tokenswap covenant between two parties.

It holds a list of parties participating in the swap with amount and denom theyre expected to provide.

If holder receives all expected tokens before the deposit deadline expires,
it forwards them to the splitter module, dequeues from the clock, and completes.

If either/both party contributions fail to reach this contract before the expiration deadline,
holder completes without dequeuing itself from the clock. This enables any late deposits
to be refunded to the parties.
