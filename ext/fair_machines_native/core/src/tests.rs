use super::*;
use crate::test_support::nz;
use matryona_machines::testing::refuses;

mod model;

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
fn every_refusal_changes_nothing() {
    let mut fair = TenantFair::new([nz(4), nz(4)], [nz(3), nz(1)]);
    assert_eq!(fair.admit(0, nz(4)), Ok(()));
    assert_eq!(fair.admit(1, nz(2)), Ok(()));
    // Mid-rotation: a paid visit and a moved cursor are part of the value.
    assert_eq!(fair.select(&heads([1, 1])), Ok(Some(0)));
    refuses(&mut fair, Refusal::Insufficient, |f| f.admit(0, nz(2)));
    refuses(&mut fair, Refusal::UnknownTenant, |f| f.admit(9, nz(1)));
    refuses(&mut fair, Refusal::ExceedsInFlight, |f| f.cancel(1, nz(3)));
    refuses(&mut fair, Refusal::UnknownTenant, |f| f.cancel(9, nz(1)));
    // A head costing more than its tenant admitted refuses the whole round.
    refuses(&mut fair, Refusal::ExceedsInFlight, |f| {
        f.select(&heads([4, 1]))
    });
    refuses(&mut fair, Refusal::ExceedsInFlight, |f| {
        f.select(&heads([0, 3]))
    });

    // A tenant that admitted nothing cannot present a head at all.
    let mut fair = TenantFair::new([nz(5), nz(5)], [nz(1); 2]);
    assert_eq!(fair.admit(1, nz(1)), Ok(()));
    refuses(&mut fair, Refusal::ExceedsInFlight, |f| {
        f.select(&heads([1, 1]))
    });
    assert_eq!(fair.select(&heads([0, 1])), Ok(Some(1)));
    assert_eq!(fair.in_flight(1), Some(0));
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
fn refusals_describe_themselves() {
    extern crate std;
    use std::string::ToString;

    let cases = [
        (
            Refusal::UnknownTenant,
            "tenant index is not one of the scheduler's tenants",
        ),
        (
            Refusal::UnknownLane,
            "lane is not one of this instance's lanes",
        ),
        (
            Refusal::Insufficient,
            "more cost asked for than the budget has available",
        ),
        (
            Refusal::ExceedsInFlight,
            "more cost returned than the budget has admitted",
        ),
    ];
    for (refusal, text) in cases {
        assert_eq!(refusal.to_string(), text);
    }
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
