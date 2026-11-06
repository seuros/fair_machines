# FairMachines

> One tenant with ten thousand jobs should not make every other tenant wait ten thousand jobs. ⚖️

Cost-aware fair scheduling for Ruby. You give each tenant a budget for how much work it may have queued and a share of each dispatch round. `TenantFair` tells you whose job goes next, charges the tenant for its cost, and refuses to admit work past the budget. The loud tenant still gets served. It just doesn't get served *instead of* everyone else.

## Quick Start

```bash
gem 'fair_machines'
```

```ruby
# Two tenants. Each may have 100 units of work queued; each gets 2 units per round.
fair = FairMachines::TenantFair.new(capacities: [100, 100], quanta: [2, 2])

# Enterprise tenant enqueues ten PDF exports at cost 5. Small tenant queues five emails at cost 1.
10.times { fair.admit(tenant: 0, cost: 5) }
5.times  { fair.admit(tenant: 1, cost: 1) }

# Each round, tell the scheduler the cost at the head of every tenant's queue.
# It answers with the tenant whose head runs now. You pop that job and run it.
fair.select([5, 1])   # => 1   the email goes first
fair.select([5, 1])   # => 1
fair.select([5, 1])   # => 1
fair.select([5, 1])   # => 1
fair.select([5, 1])   # => 0   the PDF has saved up enough share
```

Tenants are indices into the arrays you passed in. Heads are the cost of each tenant's next job, or `nil` when that tenant has nothing waiting.

## A Message to the Resistance

So you built the job queue. Priorities, retries, a dashboard. Then your biggest customer clicked "export everything" and ten thousand PDF jobs landed in the queue at once, and for the next forty minutes nobody else's password reset email went out.

You added a second queue for the big customer. Then a third for the next big customer. Then a priority flag that every customer's jobs somehow ended up setting. None of it fixes the actual problem, which is that the queue has no idea what a tenant is, let alone how much of the worker pool that tenant has already consumed.

Fairness is two numbers per tenant. How much work may this tenant have waiting at once, and how big a slice of each round does it get. With those, a ten-thousand-job export gets a fixed share of the workers and the password reset goes out in seconds, because the small tenant's slice is sitting there unused.

This gem keeps those two numbers honest. You keep the queues.

## Chapter 1: Stardate 2025.186, the AMS Shreedhar

> **AMS Shreedhar**, NCC-4207, *Shreedhar*-class convoy escort.
> The flagship does not get to spend every tenant's fuel allowance.

The escort doesn't fly the convoy. It holds the fuel ledger: how much each ship may have on request at once, and how much is pumped to each per cycle. A ship with a loud captain and a long manifest still draws from its own allowance, charged by the actual mass of what it asked for. A dinghy asking for one pallet gets its pallet on the next cycle.

Somewhere on the bridge, a junior officer suggests:

```ruby
def next_job
  queue.shift   # it's a queue, first come first served, that's fair, right?
end
```

"That," says the convoy master, "is how a dinghy starves behind a freighter."

## The Flight Plan

Each tenant is either idle or has cost queued. Admission moves cost in, dispatch and cancellation move it out, and a full budget turns new work away.

```mermaid
stateDiagram-v2
    [*] --> idle
    idle --> queued: admit within budget
    queued --> queued: admit within budget
    queued --> queued: admit over budget, refused
    queued --> queued: select, head cost charged
    queued --> idle: last head dispatched or cancelled
    queued --> queued: cancel, cost returned
    idle --> [*]
```

And the afternoon the enterprise tenant clicked "export everything":

```mermaid
sequenceDiagram
    participant E as Enterprise (tenant 0)
    participant F as TenantFair
    participant S as Small shop (tenant 1)
    participant W as Workers

    E->>F: admit cost 5, ten times
    F-->>E: true, true, true...
    S->>F: admit cost 1
    F-->>S: true

    W->>F: select([5, 1])
    F-->>W: tenant 1
    Note over W: password reset goes out

    W->>F: select([5, nil])
    F-->>W: tenant 0
    Note over W: first PDF renders

    S->>F: admit cost 1
    W->>F: select([5, 1])
    F-->>W: tenant 1
    Note over S: still seconds, not forty minutes
```

## How the Ledger Works

### Admit

Before a job is enqueued, ask whether the tenant has room for its cost. The answer is a plain boolean, and `false` means the job doesn't go on the queue.

```ruby
fair = FairMachines::TenantFair.new(capacities: [10, 5], quanta: [3, 1])

fair.admit(tenant: 1, cost: 5)   # => true   budget 5, now full
fair.admit(tenant: 1, cost: 1)   # => false  over budget
fair.admit(tenant: 2, cost: 1)   # => false  no such tenant
fair.tenant_count                # => 2
```

Over-budget is a refusal, not an error. Decide what to do with the job: reject it, hold it, tell the tenant.

### Select

Each round, pass the head cost of every tenant's queue. You get back the tenant whose head runs now, or `nil` when nobody is eligible. The chosen tenant is charged that cost against its budget, so the room comes back as the work drains.

```ruby
fair = FairMachines::TenantFair.new(capacities: [10, 5], quanta: [3, 1])
fair.admit(tenant: 0, cost: 4)
fair.admit(tenant: 1, cost: 1)

fair.select([4, 1])     # => 1
fair.select([4, nil])   # => 0
fair.select([4, nil])   # => nil   tenant 0 has nothing left queued
fair.admit(tenant: 0, cost: 7)   # => true   the dispatched 4 freed its room
```

Heads have to be truthful. A head that costs more than the tenant has admitted is not eligible and `select` returns `nil` for it.

### Share is by cost, not by count

Quanta are how much cost each tenant may dispatch per round, with leftover carried to the next round. A tenant whose head is expensive waits until it has saved up. A tenant with cheap jobs runs one every round.

```ruby
fair = FairMachines::TenantFair.new(capacities: [50, 50], quanta: [2, 2])
fair.admit(tenant: 0, cost: 40)   # heavy jobs at cost 4
fair.admit(tenant: 1, cost: 10)   # light jobs at cost 1

Array.new(10) { fair.select([4, 1]) }
# => [1, 1, 0, 1, 1, 1, 1, 0, 1, 1]
```

Same quanta, same share of the pool, and the light tenant still dispatches four jobs for every one heavy job, because the heavy job costs four times as much.

Give one tenant a bigger quantum and it gets a bigger share of the pool outright:

```ruby
fair = FairMachines::TenantFair.new(capacities: [20, 20], quanta: [3, 1])
fair.admit(tenant: 0, cost: 20)
fair.admit(tenant: 1, cost: 20)

Array.new(8) { fair.select([1, 1]) }
# => [0, 0, 0, 1, 0, 0, 0, 1]
```

### Cancel

A job that leaves the queue without being dispatched has to give its cost back, or the tenant's budget leaks until nothing is admitted.

```ruby
fair = FairMachines::TenantFair.new(capacities: [10, 5], quanta: [3, 1])
fair.admit(tenant: 0, cost: 4)

fair.cancel(tenant: 0, cost: 4)   # => true
fair.cancel(tenant: 0, cost: 1)   # => false  nothing left to cancel
fair.select([4, nil])             # => nil    the queue is empty, and so is the ledger
```

Cancelling more than was admitted is refused, and the ledger is untouched.

## The Rules of the Escort

- **1 to 16 tenants**, set by the length of `capacities:`. `quanta:` must be the same length. Empty, longer, or mismatched arrays raise `ArgumentError`.
- **Capacities, quanta, and costs are positive Integers** below `2**32`. Zero is `ArgumentError`, negative or too large is `RangeError`, floats and strings are `TypeError`. Nothing is rounded for you.
- **Heads are exactly `tenant_count` entries**, each a positive Integer or `nil`. The wrong length or a zero raises `ArgumentError`, and a float raises `TypeError`, before anything changes.
- **Bad tenant indices don't raise.** `admit` and `cancel` with an unknown tenant return `false`.
- **Frozen means frozen.** `freeze` the scheduler and `admit`, `cancel`, and `select` raise `FrozenError`. `tenant_count` still answers.
- **A tenant that drains to empty loses its saved-up share.** Leftover quantum doesn't accumulate for a tenant with nothing queued.

## What You Think You're Doing vs Reality

### You Think: "Round robin between tenants. Everybody gets a turn."
### Reality: A turn is not a share. One tenant's turn is a thumbnail, another's is a 40-page PDF.

Taking turns by job count hands the pool to whoever submits the heaviest jobs. Fairness has to be measured in what a job costs, which is why every call here carries a cost and why the scheduler charges it before it hands out the next turn.

## What's Yours

The scheduler is a ledger, not a queue. You keep a queue per tenant, call `admit` before you push, hand `select` the cost at the head of each queue, and pop the job from the tenant it names. Your cost estimates have to be honest, because the ledger believes them. And when a job dies on the floor without running, call `cancel`, or the tenant pays for it forever.

## License

MIT License. See [LICENSE.txt](LICENSE.txt) for details.

**Remember: The export job will finish. The question is whether the password reset has to wait for it.**
