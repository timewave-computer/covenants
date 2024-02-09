
```
     .-----.
  ,-'.-----.'-.
 / ,'   ∅   `. \
; ; ∅   ^   ∅ : :
| |     |     | |
| |∅    +->  ∅| |
: :           ; ;
 \ \ ∅     ∅ / /
  \ `.  ∅  ,' /
  |   `---'   |
  +-----------+
```
A clock contract for advancing a state machine. See src/msg.rs for API
documentation.

To receive ticks from this contract, import
`covenant_clock_derive::clocked` and derive `#[clocked]` on your
contract's execute message. See this contract's execute message for an
example.

To advance the clock, call `ExecuteMsg::Tick {}` on this
contract. Anyone may call this method.
