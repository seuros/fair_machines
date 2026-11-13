#![allow(clippy::expect_used)]
use super::*;
use crate::tests::nz;

#[test]
fn a_lane_cannot_borrow_another_lanes_budget() {
    let mut fair = KeyedFair::<4>::new(nz(2));
    let (hot, cold) = (0, u64::MAX);
    assert_ne!(KeyedFair::<4>::lane(hot), KeyedFair::<4>::lane(cold));
    let lane = fair.admit(hot, nz(2)).expect("within budget");
    assert_eq!(lane, KeyedFair::<4>::lane(hot));
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
    let lane = fair.admit(7, nz(1)).expect("admitted");
    let foreign = KeyedFair::<8>::lane(u64::MAX);
    let before = fair.clone();
    assert_eq!(fair.admit(7, nz(3)), Err(Refusal::Insufficient));
    assert_eq!(fair.release(lane, nz(2)), Err(Refusal::ExceedsInFlight));
    assert_eq!(fair.release(foreign, nz(1)), Err(Refusal::UnknownLane));
    assert_eq!(fair, before);
    assert_eq!(
        (fair.in_flight(foreign), fair.available(foreign)),
        (None, None)
    );
    // Lanes count cost, not work: a repeat is honoured only while cost remains.
    assert_eq!(fair.release(lane, nz(1)), Ok(()));
    let before = fair.clone();
    assert_eq!(fair.release(lane, nz(1)), Err(Refusal::ExceedsInFlight));
    assert_eq!(fair, before);
}

#[test]
fn a_lane_is_a_bucket_not_an_issued_id() {
    // Built without admitting anything, equal across instances and calls.
    let mut fair = KeyedFair::<4>::new(nz(2));
    let built = KeyedFair::<4>::lane_at(3).expect("in range");
    assert_eq!(built, KeyedFair::<4>::lane(u64::MAX));
    assert_eq!(fair.admit(u64::MAX, nz(1)), Ok(built));
    assert_eq!(KeyedFair::<4>::new(nz(9)).admit(u64::MAX, nz(1)), Ok(built));
    assert_eq!(fair.release(built, nz(1)), Ok(()));
    assert_eq!(KeyedFair::<2>::lane_at(1).map(Lane::index), Some(1));
    assert_eq!(KeyedFair::<2>::lane_at(2), None);
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
    let mut state = 0x2545_F491_4F6C_DD1D_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut fair = KeyedFair::<LANES>::new(nz(6));
    let mut counters = [0_u32; LANES];
    let mut refusals = [0_u32; 3];
    for _ in 0..4000 {
        let before = fair.clone();
        #[allow(clippy::cast_possible_truncation)]
        let cost = 1 + (next() % 4) as u32;
        let refused = if next() % 2 == 0 {
            let hash = next();
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
            let lane = KeyedFair::<8>::lane(next());
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
        if let Some(refusal) = refused {
            assert_eq!(fair, before, "{refusal:?} changed the lanes");
            refusals[match refusal {
                Refusal::Insufficient => 0,
                Refusal::ExceedsInFlight => 1,
                Refusal::UnknownLane => 2,
                Refusal::UnknownTenant => unreachable!("KeyedFair has no tenants"),
            }] += 1;
        }
        for (index, counter) in counters.iter().enumerate() {
            let lane = KeyedFair::<LANES>::lane_at(index).expect("in range");
            assert_eq!(fair.in_flight(lane), Some(*counter));
        }
    }
    assert!(refusals.iter().all(|count| *count > 0), "{refusals:?}");
}
