# Two party POL holder

## Multiple parties

Multiple parties are going to be participating, so the holder should store a list of whitelisted addresses.

## Lock Period

A `Lock` duration should be stored to keep track of the covenant duration.

After the `Lock` period expires, holder should withdraw the liquidity and forward the underlying funds to the configured splitter module that will deal with the distribution.

Splitter should be instantiated on demand, when the lock expires.

## Ragequit

A ragequit functionality should be enabled for both parties that may wish to break their part of the covenant.
Ragequitting party is subject to a percentage based penalty agreed upon instantiation.

Holder then withdraws the allocation of the ragequitting party (minus the penalty) and forwards the funds to the party.

The other party may remain in their position for as long as they wish to, but not any longer than the initial duration.
Alternatively, they may also exit their position without any penalty because the covenant is no longer valid.
In both cases, the non-ragequitting party receives their allocation plus the penalties deducted from the ragequitting party.

## Updates

Both parties are free to update their respective whitelisted addresses and do not need counterparty permission to do so.
