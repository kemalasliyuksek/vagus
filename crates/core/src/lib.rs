//! Core building blocks shared by the Vagus daemon and its modules.
//!
//! See `docs/design/vagus-architecture.md` for how the pieces fit together.

#![forbid(unsafe_code)]

mod ring_buffer;

pub use ring_buffer::RingBuffer;
