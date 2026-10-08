//! The Vagus daemon. It owns collection, state and actions; every client reaches it
//! through its API (ADR 0007).

#![forbid(unsafe_code)]

mod sampler;

pub use sampler::{Sampler, SamplerHandle};
