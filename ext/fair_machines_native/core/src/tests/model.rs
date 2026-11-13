//! Randomized run against a plain reference model.
use super::*;
use priority_machines::DeficitRoundRobin;

const N: usize = 3;
const _: () = assert!(N + 1 == 4, "the tenant draw below covers 0..=N");

/// The scheduler restated: plain integer budgets beside the sibling DRR.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Model {
    capacities: [u32; N],
    queued: [u32; N],
    drr: DeficitRoundRobin<N>,
}

impl Model {
    fn admit(&mut self, tenant: usize, cost: u32) -> Result<(), Refusal> {
        let queued = self.queued.get_mut(tenant).ok_or(Refusal::UnknownTenant)?;
        if cost > self.capacities[tenant] - *queued {
            return Err(Refusal::Insufficient);
        }
        *queued += cost;
        Ok(())
    }

    fn give_back(&mut self, tenant: usize, cost: u32) -> Result<(), Refusal> {
        let queued = self.queued.get_mut(tenant).ok_or(Refusal::UnknownTenant)?;
        if cost > *queued {
            return Err(Refusal::ExceedsInFlight);
        }
        *queued -= cost;
        if *queued == 0 {
            self.drr.deactivate(tenant).expect("a known lane");
        }
        Ok(())
    }

    fn select(&mut self, heads: &[Option<NonZeroU32>; N]) -> Result<Option<usize>, Refusal> {
        let unadmitted = |(head, queued): (&Option<NonZeroU32>, &u32)| {
            head.is_some_and(|cost| cost.get() > *queued)
        };
        if heads.iter().zip(&self.queued).any(unadmitted) {
            return Err(Refusal::ExceedsInFlight);
        }
        let Some(tenant) = self.drr.select(heads) else {
            assert!(heads.iter().all(Option::is_none));
            return Ok(None);
        };
        let cost = heads[tenant].expect("a selected head");
        self.give_back(tenant, cost.get())?;
        Ok(Some(tenant))
    }
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    #[allow(clippy::cast_possible_truncation)]
    fn below(&mut self, bound: u32) -> u32 {
        (self.next() % u64::from(bound)) as u32
    }
}

fn run(seed: u64, refusals: &mut [u32; 3], drains: &mut u32) {
    let mut rng = Rng(seed);
    let capacities: [u32; N] = core::array::from_fn(|_| 1 + rng.below(12));
    let quanta: [u32; N] = core::array::from_fn(|_| 1 + rng.below(5));
    let mut fair = TenantFair::new(capacities.map(nz), quanta.map(nz));
    let mut model = Model {
        capacities,
        queued: [0; N],
        drr: DeficitRoundRobin::new(quanta.map(nz)),
    };
    for _ in 0..300 {
        let before = fair.clone();
        let tenant = rng.below(4) as usize; // 0..=N: one past the end
        let cost = 1 + rng.below(8);
        let refused = match rng.below(4) {
            0 | 1 => {
                let answer = fair.admit(tenant, nz(cost));
                assert_eq!(answer, model.admit(tenant, cost));
                answer.err()
            }
            2 => {
                let answer = fair.cancel(tenant, nz(cost));
                assert_eq!(answer, model.give_back(tenant, cost));
                answer.err()
            }
            _ => {
                // Mostly truthful heads; one draw in four may lie.
                let heads: [_; N] = core::array::from_fn(|i| {
                    let limit = if rng.below(4) == 0 {
                        9
                    } else {
                        model.queued[i]
                    };
                    NonZeroU32::new(rng.below(limit + 1))
                });
                let answer = fair.select(&heads);
                assert_eq!(answer, model.select(&heads));
                *drains += u32::from(answer == Ok(None));
                answer.err()
            }
        };
        if let Some(refusal) = refused {
            assert_eq!(fair, before, "{refusal:?} changed the scheduler");
            refusals[match refusal {
                Refusal::UnknownTenant => 0,
                Refusal::Insufficient => 1,
                Refusal::ExceedsInFlight => 2,
                Refusal::UnknownLane => unreachable!("TenantFair has no lanes"),
            }] += 1;
        }
        for (tenant, (queued, capacity)) in model.queued.iter().zip(capacities).enumerate() {
            assert_eq!(fair.in_flight(tenant), Some(*queued));
            assert_eq!(fair.available(tenant), Some(capacity - queued));
        }
        assert_eq!(fair.scheduler, model.drr);
    }
}

#[test]
fn random_runs_match_the_model_and_refusals_change_nothing() {
    let (mut refusals, mut drains) = ([0; 3], 0);
    for seed in 1..=300_u64 {
        run(
            seed.wrapping_mul(0x9E37_79B9_7F4A_7C15),
            &mut refusals,
            &mut drains,
        );
    }
    assert!(refusals.iter().all(|count| *count > 0), "{refusals:?}");
    assert!(drains > 0);
}
