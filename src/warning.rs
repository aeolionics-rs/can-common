// SPDX-FileCopyrightText: 2026-2026. Aeolionics, LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The error-warning level, and who waits for it to change.

use core::task::{Context, Poll, Waker};

/// Whether an error counter is at the warning level, as last read and as
/// last taken by a waiter, and who waits for it to change.
#[derive(Debug, Default)]
pub struct Warning {
    now: bool,
    taken: bool,
    waker: Option<Waker>,
}

impl Warning {
    /// Not at the warning level, and no one waiting.
    #[must_use]
    pub const fn new() -> Self {
        Self { now: false, taken: false, waker: None }
    }

    /// Notes the level as read, waking the waiter on a change.
    pub fn update(&mut self, now: bool) {
        if now != self.now {
            self.now = now;
            if let Some(waker) = self.waker.take() {
                waker.wake();
            }
        }
    }

    /// The level once it differs from the one last taken; otherwise waits.
    pub fn poll_change(&mut self, cx: &Context<'_>) -> Poll<bool> {
        if self.now == self.taken {
            self.waker = Some(cx.waker().clone());
            Poll::Pending
        } else {
            self.taken = self.now;
            Poll::Ready(self.now)
        }
    }

    /// Forgets the level, as after a reset, without waking the waiter.
    pub fn forget(&mut self) {
        self.now = false;
        self.taken = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::task::Waker;

    #[test]
    fn a_change_is_taken_once() {
        let mut w = Warning::new();
        let cx = Context::from_waker(Waker::noop());
        assert_eq!(w.poll_change(&cx), Poll::Pending);
        w.update(true);
        assert_eq!(w.poll_change(&cx), Poll::Ready(true));
        assert_eq!(w.poll_change(&cx), Poll::Pending);
        w.update(false);
        w.update(true);
        assert_eq!(w.poll_change(&cx), Poll::Pending, "back where it was taken");
        w.forget();
        assert_eq!(w.poll_change(&cx), Poll::Pending);
    }
}
