#![warn(clippy::unwrap_used, clippy::expect_used)]

extern crate core;

pub mod contract;
pub mod error;
pub mod msg;
pub mod state;

#[allow(clippy::unwrap_used)]
#[cfg(test)]
pub mod suite_tests;
