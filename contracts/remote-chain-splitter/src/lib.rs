#![warn(clippy::unwrap_used, clippy::expect_used)]

extern crate core;

pub mod contract;
pub mod error;
pub mod msg;
pub mod state;
pub mod sudo;

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod suite_test;
