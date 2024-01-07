# osmo liquid pooler

Contract responsible for providing liquidity to a specified pool on osmosis dex.

The contract receives the target denoms, provides liquidity to the specified
pool, and forwards the LP tokens to the holder.

## flow

The expected state transitions are as follows:

### 1. `Instantiated`

Ticks incoming to a contract in instantiated state will loop between the following:

1. if associated note contains the proxy address, we save it and advance `ProxyCreated`

2. otherwise we submit an empty polytone message to the note which triggers the proxy creation

### 2. `ProxyCreated`

### 3. `ProxyFunded`

### 4. `Complete`
