use core::fmt;

/// Why a call was not honoured. A refused call changes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The tenant index is not one of the scheduler's tenants.
    UnknownTenant,
    /// The lane is not one of this instance's lanes: it was built for a
    /// `KeyedFair` with more lanes.
    UnknownLane,
    /// The admission asks for more cost than the budget has available.
    Insufficient,
    /// More cost is being returned than the budget accounts as admitted: an
    /// over-sized `cancel` or `release`, or a head presented to `select` that
    /// costs more than its tenant has admitted.
    ExceedsInFlight,
}

impl Refusal {
    /// A budget's refusal, under the name it has here.
    pub(crate) const fn from_flow(refusal: flow_machines::Refusal) -> Self {
        match refusal {
            flow_machines::Refusal::Insufficient { .. } => Self::Insufficient,
            flow_machines::Refusal::ExceedsInFlight { .. } => Self::ExceedsInFlight,
        }
    }

    /// The scheduler's refusal: its lanes are the tenants.
    pub(crate) const fn from_priority(refusal: priority_machines::Refusal) -> Self {
        match refusal {
            priority_machines::Refusal::UnknownLane => Self::UnknownTenant,
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::UnknownTenant => "tenant index is not one of the scheduler's tenants",
            Self::UnknownLane => "lane is not one of this instance's lanes",
            Self::Insufficient => "more cost asked for than the budget has available",
            Self::ExceedsInFlight => "more cost returned than the budget has admitted",
        })
    }
}

impl core::error::Error for Refusal {}
