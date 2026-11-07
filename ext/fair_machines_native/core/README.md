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
