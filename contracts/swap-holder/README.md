# Swap Holder

Swap Holder is a contract meant to facilitate a tokenswap covenant between two parties.

It holds a list of parties participating in the swap with amount and denom theyre expected to provide.

If holder receives all expected tokens, it forwards them to the splitter module and completes.

After a specified duration, parties that delivered their funds are allowed to withdraw (or are automatically refunded).
