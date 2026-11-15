// SPDX-FileCopyrightText: 2026-2026. Aeolionics, LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! What CAN controller drivers share, whatever traits they implement
//! (can-hal's, embedded-can's) and whatever controller they drive:
//!
//! - [`ordering`]: transmissions outstanding in the controller, reported in
//!   the order they were handed over whatever order they finish in;
//! - [`queue`]: a bounded first-in, first-out queue, `const`-constructible,
//!   for the events an interrupt handler leaves for the channel;
//! - [`timing`]: the bit-timing solver, for any controller's field ranges;
//! - [`Clock`]: the source of timestamps and timeouts a driver is given;
//! - [`Warning`]: whether an error counter is at the warning level, and who
//!   waits for it to change;
//! - [`Counts`]: running counts of frames and failures, for statistics.
//!
//! It uses no CAN trait crate's types: a driver maps [`ordering::Done`] and
//! [`timing::Segments`] to those of the traits it implements.

#![no_std]

#[cfg(test)]
extern crate std;

pub mod ordering;
pub mod queue;
pub mod timing;

mod warning;

pub use warning::Warning;

use core::time::Duration;

/// A source of time for timestamps and timeouts.
///
/// Its [`now`](Self::now) is read in the interrupt handler, to stamp the
/// frames received and the transmissions sent.
pub trait Clock {
    /// A point in time.
    type Instant: Clone + Send + 'static;
    /// The time now.
    fn now() -> Self::Instant;
    /// The time `timeout` from now.
    fn deadline(timeout: Duration) -> Self::Instant;
    /// Whether `deadline` has passed.
    fn has_passed(deadline: &Self::Instant) -> bool;
}

/// Running counts of what a channel received, sent, dropped and failed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Counts {
    /// Frames received.
    pub rx_frames: u32,
    /// Frames sent.
    pub tx_frames: u32,
    /// Frames received with no room left for them.
    pub rx_dropped: u32,
    /// Frames the controller's receive FIFO lost, full.
    pub rx_overruns: u32,
    /// Transmissions that failed.
    pub tx_failed: u32,
    /// Bus errors seen.
    pub bus_errors: u32,
}

impl Counts {
    /// All zero.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            rx_frames: 0,
            tx_frames: 0,
            rx_dropped: 0,
            rx_overruns: 0,
            tx_failed: 0,
            bus_errors: 0,
        }
    }
}
