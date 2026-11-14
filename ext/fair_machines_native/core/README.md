# fair-machines

> **AMS Shreedhar**, NCC-4207, *Shreedhar*-class convoy escort.

Engine-room flight manual: share departure capacity without starving smaller crews.

Allocation-free `no_std` tenant isolation: independent cost-credit budgets for
admission, with DRR selection delegated to `priority-machines`. This is not a
second implementation of DRR. Admission accounts queued cost; selection returns
that cost to the tenant budget. Running work after selection is not bounded here.

Adapters own queues, present truthful head costs, and serialize admission,
selection, and cancellation. Aggregate credits do not identify individual jobs.
`cancel` returns admitted cost; when a tenant drains its DRR visit is reset.
Fairness assumes continuously backlogged tenants and meaningful cost estimates.
`KeyedFair<LANES>` covers an open tenant set: caller-hashed keys map onto fixed
lanes with independent credit budgets and no scheduler. Keys sharing a lane share
its budget.

Tenant hierarchies, per-flow WFQ, runtime preemption, and strict latency/starvation
bounds are not implemented.

## Contract

- **Answers.** Every operation that can refuse returns `Result<T, Refusal>`,
  and `T` describes only honoured outcomes.

  | Call | Honoured | Refused |
  | --- | --- | --- |
  | `TenantFair::admit(tenant, cost)` | `()` | `UnknownTenant`, `Insufficient` |
  | `TenantFair::cancel(tenant, cost)` | `()` | `UnknownTenant`, `ExceedsInFlight` |
  | `TenantFair::select(heads)` | `Some(tenant)`, or `None` exactly when every head is `None` | `ExceedsInFlight` |
  | `KeyedFair::admit(hash, cost)` | the `Lane` charged | `Insufficient` |
  | `KeyedFair::release(lane, cost)` | `()` | `UnknownLane`, `ExceedsInFlight` |
  | `in_flight`, `available` | `Some(cost)`, `None` for a tenant or lane the instance does not have | queries, never refuse |
  | `KeyedFair::lane(hash)`, `lane_at(index)`, `Lane::new(index)` | a `Lane`; `lane_at` is `None` past the last lane | pure functions |

- **Refusals change nothing.** Every `Err` leaves the value equal to a clone
  taken before the call: no budget, no DRR credit, no cursor.
- **`Refusal`.** `UnknownTenant`: the index is `N` or more. `UnknownLane`: the
  lane was built for a `KeyedFair` with more lanes. `Insufficient`: the
  admission asks for more cost than the budget has available; there is no
  partial admission. `ExceedsInFlight`: more cost is being returned than the
  budget has admitted; there is no partial return.
- **`select` and an unadmitted head.** `select` is a drain: `Ok(None)` means
  every head was `None`, and nothing else. A head that costs more than its
  tenant has admitted is not "nothing to dispatch": selecting it would return
  cost that was never admitted, the same fault as an over-sized `cancel`. It
  is refused as `ExceedsInFlight` and the whole call changes nothing, rather
  than treating that tenant as empty, which would silently drop its saved
  credit and hide the caller's accounting error. An honoured `select` records
  what the round observed, as DRR does: a tenant presented as `None` loses the
  credit it had saved.
- **Repeats cannot be honoured as repeats.** Budgets count cost, not jobs, so
  the core cannot tell a repeated `cancel` or `release` from a second job of
  the same cost. Either is honoured while that much cost is in flight and
  refused as `ExceedsInFlight` once it is not. Return every admitted cost
  exactly once.
- **`Lane` is not an issued id.** It is the bucket a hash maps to, a pure
  function of the hash and `LANES`. Nothing is counted or issued: `admit`
  answers it only to save the caller a second hash, `lane`, `lane_at` and
  `Lane::new` build one without admitting anything, and the same hash names the same lane
  in every instance with as many lanes. That aliasing is the design, so a lane
  carries no scope, proves no admission, and there is no sequence to resume. A
  lane built for more lanes than an instance has is `UnknownLane`.
- **No issued ids, no restarts to manage.** Neither type hands out a handle.
  A rebuilt instance starts with empty budgets; cost admitted before the
  restart is the caller's to re-admit or forget.
- **No clock.** Nothing here takes time, so no answer records one.
- **No panics.** The one `assert!` is evaluated at compile time:
  `KeyedFair<0>` does not build, so no runtime path can meet a lane-less
  instance. Three arms the sibling types demand and the invariants rule out
  are `debug_assert!(false, ..)` plus a refusal with nothing changed, and no
  `Refusal` variant exists only to name them: in `select`, DRR selecting a
  tenant presented without a head, and returning a head's cost that was
  already checked against the tenant's budget (`ExceedsInFlight`); in
  `select` and `cancel`, the scheduler refusing a tenant the budgets know (it
  has the same `N` lanes; `UnknownTenant`). `select` decides on a copy of the
  scheduler and `cancel` on a copy of the budget, and both commit last, so
  even those arms leave the value unchanged.
- **Sibling seams.** The `flow-machines` credit windows and the
  `priority-machines` DRR are private: no sibling type appears in this API,
  so none is re-exported. Their refusals are mapped onto `Refusal` variant by
  variant.
