# osmo liquid pooler

Contract responsible for providing liquidity to a specified pool on the Osmosis dex.
Currently we only support GAMM pools where both tokens have equal weights.

The contract receives the target denoms, provides liquidity to the specified
pool, and withdraws the liquidity tokens from osmosis to this contract. The
holder is then responsible for calling this contract to redeem the LP tokens
for the underlying assets, which are then forwarder to the holder.

Works in tandem with the [osmosis liquid pooler outpost](../outpost-osmo-liquid-pooler/README.md),
in order to ensure atomic liquidity provision given the ibc nature of this design.

## flow

The expected state transitions are as follows:

### 1. `Instantiated`

Ticks incoming to a contract in instantiated state will attempt to create a
proxy account on Osmosis via [Polytone](https://github.com/DA0-DA0/polytone).
This means submitting an empty wasm `Execute` message to the note contract.
Note then relays this message to voice, which will in turn instantiate a proxy
associated with the original caller (this contract).

After proxy is created, voice will submit a callback to note. With that callback,
note associates this contract address with the created proxy address and exposes
this association via `RemoteAddress` query. Note also calls back into our contract
which is expecting a callback.

In our contract callback handler, we will query the note for our remote address.
There are two possible cases here:

First case - no address is returned. This means that something went wrong.
We do not advance our state machine and remain in `Instantiated` state.
This means that upon next tick, we will repeat this process until note returns
an address.

Second case - an address is returned. We then store that address, and advance
the state machine to `ProxyCreated`.

```md
     t      = 1
     state  = instantiated
     action = try_create_proxy

                  ───────────outgoing message flow─────────────▶

     ┌───────────────────────────────┐    ┌────────────────────────────────────┐
     │neutron                        │    │osmosis                             │
     │                               │    │                                    │
     │  ┌─────────────┐              │    │                                    │
     │  │ osmo liquid │              │    │              ┌───────┐             │
     │  │   pooler    │              │    │              │ proxy │             │
     │  └─────────────┘              │    │              └───────┘             │
     │         │                     │    │                  ▲                 │
     │         │                     │    │                  │                 │
     │         │                     │    │            1.2. create             │
     │ 1. create_proxy               │    │                  │                 │
     │         │                     │    │                  │                 │
     │         │     ┌────────┐      1.1. execute        ┌───────┐             │
     │         └────▶│  note  │──────┼─msgs = []────────▶│ voice │             │
     │               └────────┘      │    │              └───────┘             │
     │                               │    │                                    │
     │                               │    │                                    │
     │                               │    │                                    │
     │                               │    │                                    │
     └───────────────────────────────┘    └────────────────────────────────────┘

                  ◀──────────callback message flow──────────────

    ┌───────────────────────────────┐     ┌────────────────────────────────────┐
    │neutron                        │     │osmosis                             │
    │                               │     │                                    │
    │  ┌─────────────┐              │     │              ┌───────┐             │
    │  │ osmo liquid │              │     │              │ proxy │             │
    │  │   pooler    │◀───┐         │     │              └───────┘             │
    │  └─────────────┘    │         │     │                  │                 │
    │                 1.3. proxy    │     │                  │                 │
    │                   created     │     │               1. ok                │
    │                     │         │     │                  │                 │
    │                     │         │     │                  ▼                 │
    │                ┌────────┐     │     │              ┌───────┐             │
    │             ┌─▶│  note  │◀────┼─1.1. return ───────│ voice │             │
    │             │  └────────┘     │  proxy addr        └───────┘             │
    │             │       │         │     │                                    │
    │         1.2. save   │         │     │                                    │
    │       proxy address─┘         │     │                                    │
    │                               │     │                                    │
    │                               │     │                                    │
    └───────────────────────────────┘     └────────────────────────────────────┘
```

### 2. `ProxyCreated`

Ticks incoming to a contract with a created proxy will atempt to fund the proxy.
Proxy being funded is a prerequisite for providing liquidity, and we only want to
attempt providing liquidity if we have delivered all of our funds.

Because of the async nature of IBC, we need to keep things relevant for providing
liquidity up to date. One of such things are balances of our proxy account. Upon
contract instantiation, we do not store balances of the proxy account, because
we do not even have a proxy.

After proxy is created, the first attempt to deliver funds will find that proxy
balances are unknown. This triggers a proxy denom query. Via polytone, we submit
three query requests of our proxy address to our note - one for each of the
relevant denoms (e.g. ATOM, OSMO, and the relevant LP token). Once again we attach
a callback request.

```md
    t        = 2
    state    = proxy_created
    action   = try_deliver_funds
    balances = []

                 ───────────outgoing message flow─────────────▶

    ┌───────────────────────────────┐    ┌────────────────────────────────────┐
    │neutron                        │    │osmosis  1.3. execute bank balance  │
    │                               │    │           ┌──queries on self       │
    │  ┌─────────────┐              │    │           │             │          │
    │  │ osmo liquid │              │    │           │  ┌───────┐  │          │
    │  │   pooler    │              │    │           └──│ proxy │◀─┘          │
    │  └─────────────┘              │    │              └───────┘             │
    │         │                     │    │                  ▲                 │
    │ 1. query d1, d2, lp           │    │                  │                 │
    │   token balances              │    │            1.2. execute            │
    │         │                     │    │              queries               │
    │         │                  1.1. query msgs =          │                 │
    │         │     ┌────────┐      [d1_query,          ┌───────┐             │
    │         └────▶│  note  │──────┼d2_query, ────────▶│ voice │             │
    │               └────────┘      │lp_query]          └───────┘             │
    │                               │    │                                    │
    │                               │    │                                    │
    │                               │    │                                    │
    │                               │    │                                    │
    └───────────────────────────────┘    └────────────────────────────────────┘
```

Once the balances get queried on osmosis, voice submits the query results to our
note. Note then calls into our contract callback handler, in which we deserialize
the query responses and reflect the fresh balances in our storage.
```md
                  ◀──────────callback message flow──────────────

    ┌───────────────────────────────┐     ┌────────────────────────────────────┐
    │neutron                        │     │osmosis                             │
    │                               │     │                                    │
    │  ┌─────────────┐              │     │              ┌───────┐             │
    │  │ osmo liquid │              │     │              │ proxy │             │
    │  │   pooler    │◀───┐         │     │              └───────┘             │
    │  └─────────────┘    │         │     │                  │                 │
    │                 1.3. proxy    │     │              1. query              │
    │                  balances     │     │              results               │
    │                  callback     │     │                  │                 │
    │                     │         │     │                  ▼                 │
    │                ┌────────┐     │     │              ┌───────┐             │
    │                │  note  │◀────┼──1.1. query ───────│ voice │             │
    │                └────────┘     │   results          └───────┘             │
    │                               │     │                                    │
    │                               │     │                                    │
    │                               │     │                                    │
    │                               │     │                                    │
    │                               │     │                                    │
    └───────────────────────────────┘     └────────────────────────────────────┘
```

After balances are fresh - we once again try to deliver funds. This time we can
see that our proxy has no/insufficient tokens for our desired liquidity provision.

We then query our own balances of relevant tokens, and attempt to transfer them
directly to our proxy over ibc, without using polytone. Along with submitting
these ibc send messages, we clear our latest proxy balances storage to re-trigger
a query on next tick.

```md
    t        = 3
    state    = proxy_created
    action   = try_deliver_funds
    balances = [(d1, 0), (d2, 0), (lp, 0)]

                 ───────────outgoing message flow─────────────▶

    ┌───────────────────────────────┐    ┌────────────────────────────────────┐
    │neutron                        │    │osmosis                             │
    │                               │    │                                    │
    │                               │    │                                    │
    │                               │    │                                    │
    │                               │    │                                    │
    │      ┌─────────────┐          │    │                                    │
    │      │ osmo liquid │         1. ibc send          ┌───────┐             │
    │   ┌─▶│   pooler    │─────────(d1, 100) + ────────▶│ proxy │             │
    │   │  └─────────────┘          (d2, 120)           └───────┘             │
    │   │         │                 │    │                                    │
    │   1.1. clear│                 │    │                                    │
    │   └─latest ─┘                 │    │                                    │
    │    balances                   │    │                                    │
    │                               │    │                                    │
    │                               │    │                                    │
    │                               │    │                                    │
    │                               │    │                                    │
    │                               │    │                                    │
    └───────────────────────────────┘    └────────────────────────────────────┘
```

Upon next tick, regardless of the ibc transfer outcome, the contract will not
have any knowledge of the proxy balances as we erased them in the previous step.
This will trigger the same query process as we performed upon entering `ProxyCreated`
state.

Once the proxy balances query concludes its round trip, the next tick coming into
the liquid pooler will assert whether the proxy balances are what we expect them to
be in order to begin the liquidity provision process. If that is the case, we advance
the contract state to `ProxyFunded`. If balances are insufficient, we will try to
fund the proxy over ibc again, restarting the flow.

### 3. `ProxyFunded`

Ticks incoming to a contract with a funded proxy will attempt to provide liquidity.
This means two things.

First, that we construct a polytone message that will be sent to our osmosis outpost.
This message will contain all balances that the proxy holds, and will attempt to
provide liquidity according to our config.

The second message will once again trigger a balances query.

We submit these two messages in order: first we attempt to provide liquidity, and
after that we query the proxy balances. If providing liquidity succeeded, we will
see that reflected by the available gamm token (and reduced target denoms) balance.

This process repeats for as long as the liquid pooler is configured to with the
`funding_expiration` field.

```md
     t      = 4
     state  = proxy_funded
     action = try_provide_liquidity

                  ───────────outgoing message flow─────────────▶

     ┌───────────────────────────────┐    ┌────────────────────────────────────┐
     │neutron                        │    │osmosis  ┌──1.5. return lp ──┐      │
     │                               │    │         │      tokens       │      │
     │  ┌─────────────┐              │    │ 1.6.─┐  │                   │      │
     │  │ osmo liquid │              │    │query │  ▼               ┌───────┐  │
     │  │   pooler    │              │    │ bals┌┴──────┐ 1.3. enter│ osmo  │  │
     │  └─────────────┘              │    │ └──▶│ proxy │────pool──▶│outpost│  │
     │         │                     │    │     └───────┘           └───────┘  │
     │         │                     │    │         ▲                   │      │
     │ 1. LP and query               │    │         │                   │      │
     │    balances                   │    │         └───┐         1.4. provide │
     │         │                     │    │      1.2. lp, query     liquidity  │
     │         │                     │    │             │               │      │
     │         │  ┌────────┐         │ 1.1.         ┌───────┐           ▼      │
     │         └─▶│  note  │───execute msgs = [lp]─▶│ voice │     ┌───────────┐│
     │            └────────┘   query msgs = [query] └───────┘     │ gamm pool ││
     │                               │    │                       └───────────┘│
     │                               │    │                                    │
     │                               │    │                                    │
     └───────────────────────────────┘    └────────────────────────────────────┘

                  ◀──────────callback message flow──────────────

    ┌───────────────────────────────┐     ┌────────────────────────────────────┐
    │neutron                        │     │osmosis                             │
    │                               │     │                                    │
    │  ┌─────────────┐              │     │              ┌───────┐             │
    │  │ osmo liquid │              │     │              │ proxy │             │
    │  │   pooler    │◀───┐         │     │              └───────┘             │
    │  └─────────────┘  1.2. lp     │     │                  │                 │
    │                 callback +    │     │                 1.                 │
    │                  balances     │     │              balances              │
    │                   update      │     │              response              │
    │                     │         │     │                  ▼                 │
    │                ┌────────┐     │     │              ┌───────┐             │
    │                │  note  │◀────┼1.1. balances ──────│ voice │             │
    │                └────────┘     │   response         └───────┘             │
    │                               │     │                                    │
    │                               │     │                                    │
    │                               │     │                                    │
    │                               │     │                                    │
    │                               │     │                                    │
    └───────────────────────────────┘     └────────────────────────────────────┘
```
