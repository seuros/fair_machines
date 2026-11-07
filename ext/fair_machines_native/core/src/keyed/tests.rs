#![allow(clippy::expect_used)]
use super::*;
use crate::tests::nz;

#[test]
fn a_lane_cannot_borrow_another_lanes_budget() {
    let mut fair = KeyedFair::<4>::new(nz(2));
    let (hot, cold) = (0, u64::MAX);
    assert_ne!(KeyedFair::<4>::lane(hot), KeyedFair::<4>::lane(cold));
    let lane = fair.admit(hot, nz(2)).expect("within budget");
    assert_eq!(fair.admit(hot, nz(1)), None);
    assert_eq!(fair.in_flight(lane), 2);
    assert!(fair.admit(cold, nz(2)).is_some());
    assert!(fair.release(lane, nz(1)));
    assert_eq!(fair.admit(hot, nz(1)), Some(lane));
}

#[test]
fn over_release_and_foreign_lanes_change_nothing() {
    let mut fair = KeyedFair::<2>::new(nz(3));
    let lane = fair.admit(7, nz(1)).expect("admitted");
    assert!(!fair.release(lane, nz(2)));
    assert!(!fair.release(Lane(2), nz(1)));
    assert_eq!(fair.in_flight(Lane(2)), 0);
    assert_eq!(KeyedFair::<2>::lane_at(1), Some(Lane(1)));
    assert_eq!(KeyedFair::<2>::lane_at(2), None);
    assert!(fair.release(lane, nz(1)));
    assert!(!fair.release(lane, nz(1)));
}

#[test]
fn lanes_cover_the_hash_space_in_order() {
    assert_eq!(KeyedFair::<1>::lane(u64::MAX).index(), 0);
    let quarters = [0, 1 << 62, 1 << 63, u64::MAX].map(|h| KeyedFair::<4>::lane(h).index());
    assert_eq!(quarters, [0, 1, 2, 3]);
}
