#![no_std]
#![doc = include_str!("../README.md")]

mod deficit;
mod edf;
mod weighted;

pub use deficit::DeficitRoundRobin;
pub use edf::earliest_deadline;
pub use weighted::WeightedRoundRobin;
