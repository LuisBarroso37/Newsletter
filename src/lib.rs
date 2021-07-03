#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

pub mod configuration;
pub mod domain;
pub mod email_client;
pub mod routes;
pub mod startup;
pub mod telemetry;
