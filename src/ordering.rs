// SPDX-FileCopyrightText: 2026-2026. Aeolionics, LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Transmissions outstanding in a controller's `SLOTS` transmit buffers
//! (mailboxes), reported in the order they were handed over whatever order
//! they finish in, as a confirmed-transmission channel needs: a frame
//! aborted or failed in one buffer waits for an older one still on its way
//! in another.

/// How a transmission ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Done<T> {
    /// Sent, at the time given.
    Sent(T),
    /// Taken back before it was sent, as asked.
    Aborted,
    /// The controller gave up on it: single-shot, or going bus-off.
    GaveUp,
}

/// What [`Outstanding::finish`] found finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Finished<I> {
    /// The transmission's id.
    pub id: I,
    /// Whether the controller gave up on it ([`Done::GaveUp`]).
    pub gave_up: bool,
}

struct Entry<I, T> {
    id: I,
    slot: u8,
    abort_requested: bool,
    done: Option<Done<T>>,
}

/// The transmissions handed over and not yet reported, oldest first, each
/// with the transmit buffer (`slot`) it went into and an id of the driver's
/// choosing.
pub struct Outstanding<I, T, const SLOTS: usize> {
    entries: [Option<Entry<I, T>>; SLOTS],
    head: usize,
    len: usize,
}

impl<I: Copy + PartialEq, T, const SLOTS: usize> Outstanding<I, T, SLOTS> {
    /// None outstanding.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: [const { None }; SLOTS],
            head: 0,
            len: 0,
        }
    }

    /// Transmissions outstanding, finished or not.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether none are.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Whether as many are outstanding as there are buffers.
    #[must_use]
    pub const fn is_full(&self) -> bool {
        self.len == SLOTS
    }

    /// Records transmission `id`, just placed in `slot`.
    ///
    /// # Panics
    ///
    /// If [`is_full`](Self::is_full).
    pub fn push(&mut self, id: I, slot: u8) {
        assert!(!self.is_full(), "no room for another transmission");
        self.entries[(self.head + self.len) % SLOTS] = Some(Entry { id, slot, abort_requested: false, done: None });
        self.len += 1;
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = &mut Entry<I, T>> {
        let (head, len) = (self.head, self.len);
        let (front, back) = self.entries.split_at_mut(head);
        back.iter_mut().chain(front.iter_mut()).take(len).filter_map(Option::as_mut)
    }

    /// Notes that transmission `id` is to be aborted, returning the slot to
    /// abort if it is still unfinished.
    pub fn request_abort(&mut self, id: I) -> Option<u8> {
        let entry = self.iter_mut().find(|e| e.id == id && e.done.is_none())?;
        entry.abort_requested = true;
        Some(entry.slot)
    }

    /// Notes that every unfinished transmission is to be aborted, returning
    /// their slots.
    pub fn abort_unfinished(&mut self) -> impl Iterator<Item = u8> + '_ {
        self.iter_mut().filter(|e| e.done.is_none()).map(|e| {
            e.abort_requested = true;
            e.slot
        })
    }

    /// The slots of the transmissions still unfinished.
    pub fn unfinished(&mut self) -> impl Iterator<Item = u8> + '_ {
        self.iter_mut().filter(|e| e.done.is_none()).map(|e| e.slot)
    }

    /// The transmission in `slot` has finished: `sent` with its time if it
    /// was transmitted. If not, it was aborted when that was asked for, or
    /// else the controller gave up on it. Returns which it was, or `None` if
    /// no unfinished transmission is in `slot`.
    pub fn finish(&mut self, slot: u8, sent: Option<T>) -> Option<Finished<I>> {
        // A slot is reused once it has finished, so it's the oldest
        // unfinished transmission in it that finished now.
        let entry = self.iter_mut().find(|e| e.slot == slot && e.done.is_none())?;
        let done = match sent {
            Some(time) => Done::Sent(time),
            None if entry.abort_requested => Done::Aborted,
            None => Done::GaveUp,
        };
        let gave_up = matches!(done, Done::GaveUp);
        entry.done = Some(done);
        Some(Finished { id: entry.id, gave_up })
    }

    /// The report of the oldest transmission, if it has finished.
    pub fn pop_report(&mut self) -> Option<(I, Done<T>)> {
        if self.len == 0 {
            return None;
        }
        let slot = &mut self.entries[self.head];
        if slot.as_ref().is_none_or(|e| e.done.is_none()) {
            return None;
        }
        let entry = slot.take()?;
        self.head = (self.head + 1) % SLOTS;
        self.len -= 1;
        Some((entry.id, entry.done?))
    }
}

impl<I: Copy + PartialEq, T, const SLOTS: usize> Default for Outstanding<I, T, SLOTS> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[expect(clippy::cast_possible_truncation, reason = "small test counts")]
mod tests {
    use super::*;
    use std::vec::Vec;

    const SLOTS: usize = 3;

    fn reports(outstanding: &mut Outstanding<u32, u32, SLOTS>) -> Vec<(u32, Done<u32>)> {
        core::iter::from_fn(|| outstanding.pop_report()).collect()
    }

    #[test]
    fn reports_in_hand_over_order() {
        let mut o = Outstanding::new();
        o.push(1, 0);
        o.push(2, 1);
        o.push(3, 2);
        assert!(o.is_full());
        // B aborted while A is on its way: B's report waits for A's.
        assert_eq!(o.request_abort(2), Some(1));
        assert_eq!(o.finish(1, None), Some(Finished { id: 2, gave_up: false }));
        assert_eq!(reports(&mut o), []);
        o.finish(0, Some(100));
        o.finish(2, Some(200));
        assert_eq!(reports(&mut o), [(1, Done::Sent(100)), (2, Done::Aborted), (3, Done::Sent(200))]);
        assert!(o.is_empty());
    }

    #[test]
    fn a_reused_slot_finishes_its_oldest_transmission() {
        let mut o = Outstanding::new();
        o.push(1, 0);
        o.push(2, 1);
        assert_eq!(o.finish(1, None), Some(Finished { id: 2, gave_up: true }));
        o.push(3, 1); // C took B's slot
        o.finish(1, Some(30));
        o.finish(0, Some(10));
        assert_eq!(reports(&mut o), [(1, Done::Sent(10)), (2, Done::GaveUp), (3, Done::Sent(30))]);
        assert_eq!(o.finish(1, None), None, "nothing left in the slot");
    }

    #[test]
    fn an_abort_too_late_is_reported_sent() {
        let mut o = Outstanding::<u32, u32, SLOTS>::new();
        o.push(7, 2);
        assert_eq!(o.request_abort(7), Some(2));
        o.finish(2, Some(5));
        assert_eq!(reports(&mut o), [(7, Done::Sent(5))]);
        assert_eq!(o.request_abort(7), None);
        assert_eq!(o.request_abort(8), None);
    }

    #[test]
    fn wraps_around() {
        let mut o = Outstanding::new();
        for id in 0..3 * SLOTS as u32 {
            let slot = (id % SLOTS as u32) as u8;
            o.push(id, slot);
            o.finish(slot, Some(id));
            assert_eq!(reports(&mut o), [(id, Done::Sent(id))]);
        }
        let mut unfinished = Outstanding::<u32, u32, SLOTS>::new();
        unfinished.push(0, 0);
        unfinished.push(1, 1);
        unfinished.finish(0, Some(0));
        assert_eq!(unfinished.unfinished().collect::<Vec<_>>(), [1]);
        assert_eq!(unfinished.abort_unfinished().collect::<Vec<_>>(), [1]);
        unfinished.finish(1, None);
        assert_eq!(core::iter::from_fn(|| unfinished.pop_report()).collect::<Vec<_>>(), [(0, Done::Sent(0)), (1, Done::Aborted)]);
    }
}
