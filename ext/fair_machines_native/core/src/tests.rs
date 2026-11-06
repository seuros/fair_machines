#![allow(clippy::expect_used)]
use super::*;
fn nz(x: u32) -> NonZeroU32 {
    NonZeroU32::new(x).expect("positive")
}

#[test]
fn tenant_capacity_cannot_be_stolen_and_unadmitted_work_is_ineligible() {
    let mut fair = TenantFair::new([nz(2), nz(10)], [nz(1); 2]);
    assert!(fair.admit(0, nz(2)));
    assert!(!fair.admit(0, nz(1)));
    assert!(!fair.admit(2, nz(1)));
    assert_eq!(fair.select(&[None, Some(nz(1))]), None);
    assert!(fair.admit(1, nz(1)));
    assert_eq!(fair.select(&[Some(nz(1)); 2]), Some(0));
    assert!(fair.cancel(0, nz(1)));
    assert!(!fair.cancel(0, nz(1)));
    assert_eq!(fair.select(&[None, Some(nz(1))]), Some(1));
}

#[test]
fn cost_fairness_reuses_drr_and_cancelled_tenants_can_refill() {
    let mut fair = TenantFair::new([nz(100); 2], [nz(4), nz(2)]);
    assert!(fair.admit(0, nz(100)));
    assert!(fair.admit(1, nz(100)));
    let turns: [_; 6] = core::array::from_fn(|_| fair.select(&[Some(nz(2)); 2]));
    assert_eq!(
        turns,
        [Some(0), Some(0), Some(1), Some(0), Some(0), Some(1)]
    );
    assert!(fair.cancel(0, nz(92)));
    assert!(fair.admit(0, nz(2)));
    assert!(fair.select(&[Some(nz(2)); 2]).is_some());
}
