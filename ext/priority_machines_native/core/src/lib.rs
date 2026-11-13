#![no_std]
#![doc = include_str!("../README.md")]

mod deficit;
mod edf;
mod refusal;
mod round_robin;
mod weighted;

pub use deficit::DeficitRoundRobin;
pub use edf::earliest_deadline;
pub use refusal::{Deactivation, Refusal};
pub use round_robin::RoundRobin;
pub use weighted::WeightedRoundRobin;
