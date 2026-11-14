use super::*;
use crate::test_support::{Refusals, below, nz};
use matryona_machines::testing::{XorShift64, refuses};

/// The lane at `index` of a `KeyedFair<LANES>`, which must have it.
fn lane_at<const LANES: usize>(index: usize) -> Lane {
    let Some(lane) = KeyedFair::<LANES>::lane_at(index) else {
        panic!("{index} is a lane of {LANES}");
    };
    lane
}

#[test]
fn a_lane_cannot_borrow_another_lanes_budget() {
    let mut fair = KeyedFair::<4>::new(nz(2));
    let (hot, cold) = (0, u64::MAX);
    assert_ne!(KeyedFair::<4>::lane(hot), KeyedFair::<4>::lane(cold));
    let lane = KeyedFair::<4>::lane(hot);
    assert_eq!(fair.admit(hot, nz(2)), Ok(lane));
    assert_eq!(fair.admit(hot, nz(1)), Err(Refusal::Insufficient));
    assert_eq!(
        (fair.in_flight(lane), fair.available(lane)),
        (Some(2), Some(0))
    );
    assert_eq!(fair.admit(cold, nz(2)), Ok(KeyedFair::<4>::lane(cold)));
    assert_eq!(fair.release(lane, nz(1)), Ok(()));
    assert_eq!(fair.admit(hot, nz(1)), Ok(lane));
    assert_eq!(fair.lane_count(), 4);
}

#[test]
fn every_refusal_changes_nothing() {
    let mut fair = KeyedFair::<2>::new(nz(3));
    let lane = KeyedFair::<2>::lane(7);
    assert_eq!(fair.admit(7, nz(1)), Ok(lane));
    let foreign = KeyedFair::<8>::lane(u64::MAX);
    refuses(&mut fair, Refusal::Insufficient, |f| f.admit(7, nz(3)));
    refuses(&mut fair, Refusal::ExceedsInFlight, |f| {
        f.release(lane, nz(2))
    });
    refuses(&mut fair, Refusal::UnknownLane, |f| {
        f.release(foreign, nz(1))
    });
    assert_eq!(
        (fair.in_flight(foreign), fair.available(foreign)),
        (None, None)
    );
    // Lanes count cost, not work: a repeat is honoured only while cost remains.
    assert_eq!(fair.release(lane, nz(1)), Ok(()));
    refuses(&mut fair, Refusal::ExceedsInFlight, |f| {
        f.release(lane, nz(1))
    });
}

#[test]
fn lanes_cover_the_hash_space_in_order() {
    assert_eq!(KeyedFair::<1>::lane(u64::MAX).index(), 0);
    let quarters = [0, 1 << 62, 1 << 63, u64::MAX].map(|h| KeyedFair::<4>::lane(h).index());
    assert_eq!(quarters, [0, 1, 2, 3]);
    let thirds = [0, u64::MAX / 3, u64::MAX / 3 + 1, u64::MAX].map(KeyedFair::<3>::lane);
    assert_eq!(thirds.map(Lane::index), [0, 0, 1, 2]);
}

#[test]
fn random_runs_match_plain_counters_and_refusals_change_nothing() {
    const LANES: usize = 5;
    let mut rng = XorShift64::new(0x2545_F491_4F6C_DD1D);
    let mut fair = KeyedFair::<LANES>::new(nz(6));
    let mut counters = [0_u32; LANES];
    let mut refusals = Refusals::default();
    for _ in 0..4000 {
        let before = fair.clone();
        let cost = 1 + below(&mut rng, 4);
        let refused = if below(&mut rng, 2) == 0 {
            let hash = rng.next_u64();
            let lane = KeyedFair::<LANES>::lane(hash);
            let expected = if cost > 6 - counters[lane.index()] {
                Err(Refusal::Insufficient)
            } else {
                counters[lane.index()] += cost;
                Ok(lane)
            };
            let answer = fair.admit(hash, nz(cost));
            assert_eq!(answer, expected);
            answer.err()
        } else {
            let lane = KeyedFair::<8>::lane(rng.next_u64());
            let expected = match counters.get_mut(lane.index()) {
                None => Err(Refusal::UnknownLane),
                Some(counter) if cost > *counter => Err(Refusal::ExceedsInFlight),
                Some(counter) => {
                    *counter -= cost;
                    Ok(())
                }
            };
            let answer = fair.release(lane, nz(cost));
            assert_eq!(answer, expected);
            answer.err()
        };
        refusals.record(refused, &fair, &before);
        for (index, counter) in counters.iter().enumerate() {
            assert_eq!(fair.in_flight(lane_at::<LANES>(index)), Some(*counter));
        }
    }
    let Refusals {
        unknown_tenant,
        unknown_lane,
        insufficient,
        exceeds_in_flight,
    } = refusals;
    assert_eq!(unknown_tenant, 0, "KeyedFair has no tenants");
    assert!(unknown_lane > 0 && insufficient > 0 && exceeds_in_flight > 0);
}

#[test]
fn a_lane_is_a_bucket_not_an_issued_id() {
    // Built without admitting anything, equal across instances, calls and
    // constructors.
    let built = lane_at::<4>(3);
    assert_eq!(built, KeyedFair::<4>::lane(u64::MAX));
    assert_eq!(built, Lane::new(3));
    assert_eq!(Lane::new(usize::MAX).index(), usize::MAX);
    assert_eq!(KeyedFair::<2>::lane_at(2), None);
    let mut fair = KeyedFair::<4>::new(nz(2));
    assert_eq!(fair.admit(u64::MAX, nz(1)), Ok(built));
    assert_eq!(KeyedFair::<4>::new(nz(9)).admit(u64::MAX, nz(1)), Ok(built));
    assert_eq!(fair.release(built, nz(1)), Ok(()));
    // A position past the instance's lanes names nothing in it.
    refuses(&mut fair, Refusal::UnknownLane, |f| {
        f.release(Lane::new(4), nz(1))
    });
    assert_eq!(fair.in_flight(Lane::new(4)), None);
}
