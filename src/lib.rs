#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

pub mod authentication;
pub mod configuration;
pub mod domain;
pub mod email_client;
pub mod routes;
pub mod session_state;
pub mod startup;
pub mod telemetry;
pub mod utils;
