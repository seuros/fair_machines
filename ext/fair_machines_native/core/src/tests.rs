#![allow(clippy::expect_used)]
use super::*;

mod model;

pub fn nz(x: u32) -> NonZeroU32 {
    NonZeroU32::new(x).expect("positive")
}

fn heads<const N: usize>(costs: [u32; N]) -> [Option<NonZeroU32>; N] {
    costs.map(NonZeroU32::new)
}

#[test]
fn tenant_capacity_cannot_be_stolen() {
    let mut fair = TenantFair::new([nz(2), nz(10)], [nz(1); 2]);
    assert_eq!(fair.tenant_count(), 2);
    assert_eq!(fair.admit(0, nz(2)), Ok(()));
    assert_eq!(fair.admit(0, nz(1)), Err(Refusal::Insufficient));
    assert_eq!(fair.admit(2, nz(1)), Err(Refusal::UnknownTenant));
    assert_eq!((fair.in_flight(0), fair.available(0)), (Some(2), Some(0)));
    assert_eq!((fair.in_flight(1), fair.available(1)), (Some(0), Some(10)));
    assert_eq!((fair.in_flight(2), fair.available(2)), (None, None));
    assert_eq!(fair.admit(1, nz(1)), Ok(()));
    assert_eq!(fair.select(&heads([1, 1])), Ok(Some(0)));
    assert_eq!(fair.cancel(0, nz(1)), Ok(()));
    assert_eq!(fair.cancel(0, nz(1)), Err(Refusal::ExceedsInFlight));
    assert_eq!(fair.cancel(2, nz(1)), Err(Refusal::UnknownTenant));
    assert_eq!(fair.select(&heads([0, 1])), Ok(Some(1)));
    assert_eq!(fair.select(&heads([0, 0])), Ok(None));
}

#[test]
fn an_unadmitted_head_refuses_the_whole_selection() {
    let mut fair = TenantFair::new([nz(5), nz(5)], [nz(1); 2]);
    assert_eq!(fair.admit(1, nz(1)), Ok(()));
    let before = fair.clone();
    // Tenant 0 admitted nothing; tenant 1 admitted 1 and presents 2.
    assert_eq!(fair.select(&heads([1, 1])), Err(Refusal::ExceedsInFlight));
    assert_eq!(fair.select(&heads([0, 2])), Err(Refusal::ExceedsInFlight));
    assert_eq!(fair, before);
    assert_eq!(fair.select(&heads([0, 1])), Ok(Some(1)));
    assert_eq!(fair.in_flight(1), Some(0));
}

#[test]
fn every_refusal_changes_nothing() {
    let mut fair = TenantFair::new([nz(4), nz(4)], [nz(3), nz(1)]);
    assert_eq!(fair.admit(0, nz(4)), Ok(()));
    assert_eq!(fair.admit(1, nz(2)), Ok(()));
    // Mid-rotation: a paid visit and a moved cursor are part of the value.
    assert_eq!(fair.select(&heads([1, 1])), Ok(Some(0)));
    let before = fair.clone();
    assert_eq!(fair.admit(0, nz(2)), Err(Refusal::Insufficient));
    assert_eq!(fair.admit(9, nz(1)), Err(Refusal::UnknownTenant));
    assert_eq!(fair.cancel(1, nz(3)), Err(Refusal::ExceedsInFlight));
    assert_eq!(fair.cancel(9, nz(1)), Err(Refusal::UnknownTenant));
    assert_eq!(fair.select(&heads([4, 1])), Err(Refusal::ExceedsInFlight));
    assert_eq!(fair, before);
}

#[test]
fn cost_fairness_reuses_drr_and_cancelled_tenants_can_refill() {
    let mut fair = TenantFair::new([nz(100); 2], [nz(4), nz(2)]);
    assert_eq!(fair.admit(0, nz(100)), Ok(()));
    assert_eq!(fair.admit(1, nz(100)), Ok(()));
    let turns: [_; 6] = core::array::from_fn(|_| fair.select(&heads([2, 2])));
    assert_eq!(turns, [0, 0, 1, 0, 0, 1].map(|tenant| Ok(Some(tenant))));
    assert_eq!(fair.cancel(0, nz(92)), Ok(()));
    assert_eq!(fair.in_flight(0), Some(0));
    assert_eq!(fair.admit(0, nz(2)), Ok(()));
    assert!(fair.select(&heads([2, 2])).is_ok_and(|turn| turn.is_some()));
}

#[test]
fn a_drained_tenant_loses_its_saved_credit() {
    let mut fair = TenantFair::new([nz(20); 2], [nz(5); 2]);
    // Tenant 0 is paid 5, dispatches 1 and drains with 4 unspent.
    assert_eq!(fair.admit(0, nz(1)), Ok(()));
    assert_eq!(fair.select(&heads([1, 0])), Ok(Some(0)));
    assert_eq!(fair.admit(0, nz(9)), Ok(()));
    assert_eq!(fair.admit(1, nz(15)), Ok(()));
    // With the 4 kept, tenant 0 could afford its 9 on its next visit (4 + 5)
    // and would take the second turn. It was dropped, so tenant 1 does.
    let turns: [_; 3] = core::array::from_fn(|_| fair.select(&heads([9, 5])));
    assert_eq!(turns, [1, 1, 0].map(|tenant| Ok(Some(tenant))));
}

#[test]
fn refusal_is_an_error_with_distinct_variants() {
    let all = [
        Refusal::UnknownTenant,
        Refusal::UnknownLane,
        Refusal::Insufficient,
        Refusal::ExceedsInFlight,
    ];
    let error: &dyn core::error::Error = &all[0];
    assert!(error.source().is_none());
    for (i, refusal) in all.iter().enumerate() {
        for other in &all[i + 1..] {
            assert_ne!(refusal, other);
        }
    }
    let insufficient = flow_machines::Refusal::Insufficient { available: 0 };
    let exceeds = flow_machines::Refusal::ExceedsInFlight { in_flight: 0 };
    assert_eq!(Refusal::from_flow(insufficient), Refusal::Insufficient);
    assert_eq!(Refusal::from_flow(exceeds), Refusal::ExceedsInFlight);
    assert_eq!(
        Refusal::from_priority(priority_machines::Refusal::UnknownLane),
        Refusal::UnknownTenant
    );
}

#[test]
fn cancelling_an_unvisited_tenants_whole_queue_keeps_its_place_in_the_rotation() {
    let mut fair = TenantFair::new([nz(4), nz(4)], [nz(1); 2]);
    let fresh = fair.clone();
    // Tenant 0 is next but has not been paid: draining it moves nothing.
    assert_eq!(fair.admit(0, nz(1)), Ok(()));
    assert_eq!(fair.cancel(0, nz(1)), Ok(()));
    assert_eq!(fair, fresh);
    assert_eq!(fair.admit(0, nz(1)), Ok(()));
    assert_eq!(fair.admit(1, nz(1)), Ok(()));
    assert_eq!(fair.select(&heads([1, 1])), Ok(Some(0)));
}
