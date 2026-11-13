use core::fmt;

/// Why a call was not honoured. A refused call changes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The lane index is not one of the scheduler's lanes.
    UnknownLane,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownLane => f.write_str("lane index is not one of the scheduler's lanes"),
        }
    }
}

impl core::error::Error for Refusal {}

/// What an honoured `deactivate` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deactivation {
    /// The lane gave up what it held: its unused turns or credit, and the
    /// visit it was in the middle of.
    Deactivated,
    /// The lane held nothing, so it is already deactivated and nothing
    /// changed. This is what repeating a `deactivate` answers.
    Idle,
}
