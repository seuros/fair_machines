//! Randomized run against a plain reference model.
use super::*;
use crate::test_support::{Refusals, below, index};
use matryona_machines::testing::XorShift64;
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
            assert!(self.drr.deactivate(tenant).is_ok(), "a known lane");
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
        let Some(cost) = heads[tenant] else {
            panic!("DRR selected a lane without a head");
        };
        self.give_back(tenant, cost.get())?;
        Ok(Some(tenant))
    }
}

fn run(seed: u64, refusals: &mut Refusals, drains: &mut u32) {
    let mut rng = XorShift64::new(seed);
    let capacities: [u32; N] = core::array::from_fn(|_| 1 + below(&mut rng, 12));
    let quanta: [u32; N] = core::array::from_fn(|_| 1 + below(&mut rng, 5));
    let mut fair = TenantFair::new(capacities.map(nz), quanta.map(nz));
    let mut model = Model {
        capacities,
        queued: [0; N],
        drr: DeficitRoundRobin::new(quanta.map(nz)),
    };
    for _ in 0..300 {
        let before = fair.clone();
        let tenant = index(&mut rng, 4); // 0..=N: one past the end
        let cost = 1 + below(&mut rng, 8);
        let refused = match below(&mut rng, 4) {
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
                    let limit = if below(&mut rng, 4) == 0 {
                        9
                    } else {
                        model.queued[i]
                    };
                    NonZeroU32::new(below(&mut rng, limit + 1))
                });
                let answer = fair.select(&heads);
                assert_eq!(answer, model.select(&heads));
                *drains += u32::from(answer == Ok(None));
                answer.err()
            }
        };
        refusals.record(refused, &fair, &before);
        for (tenant, (queued, capacity)) in model.queued.iter().zip(capacities).enumerate() {
            assert_eq!(fair.in_flight(tenant), Some(*queued));
            assert_eq!(fair.available(tenant), Some(capacity - queued));
        }
        assert_eq!(fair.scheduler, model.drr);
    }
}

#[test]
fn random_runs_match_the_model_and_refusals_change_nothing() {
    let (mut refusals, mut drains) = (Refusals::default(), 0);
    for seed in 1..=300_u64 {
        run(
            seed.wrapping_mul(0x9E37_79B9_7F4A_7C15),
            &mut refusals,
            &mut drains,
        );
    }
    let Refusals {
        unknown_tenant,
        unknown_lane,
        insufficient,
        exceeds_in_flight,
    } = refusals;
    assert_eq!(unknown_lane, 0, "TenantFair has no lanes");
    assert!(unknown_tenant > 0 && insufficient > 0 && exceeds_in_flight > 0);
    assert!(drains > 0);
}
