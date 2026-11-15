// SPDX-FileCopyrightText: 2026-2026. Aeolionics, LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A bounded first-in, first-out queue.

/// A bounded first-in, first-out queue of up to `N` items, which a `static`
/// can hold (its [`new`](Self::new) is `const`).
pub struct Queue<E, const N: usize> {
    items: [Option<E>; N],
    head: usize,
    len: usize,
}

impl<E, const N: usize> Queue<E, N> {
    /// An empty queue.
    #[must_use]
    pub const fn new() -> Self {
        Self { items: [const { None }; N], head: 0, len: 0 }
    }

    /// Items queued.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether nothing is queued.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Room left.
    #[must_use]
    pub const fn free(&self) -> usize {
        N - self.len
    }

    /// Adds `item` at the back.
    ///
    /// # Errors
    ///
    /// Hands `item` back if the queue is full.
    pub fn push(&mut self, item: E) -> Result<(), E> {
        if self.len == N {
            return Err(item);
        }
        self.items[(self.head + self.len) % N] = Some(item);
        self.len += 1;
        Ok(())
    }

    /// Takes the item at the front.
    pub fn pop(&mut self) -> Option<E> {
        if self.len == 0 {
            return None;
        }
        let item = self.items[self.head].take();
        self.head = (self.head + 1) % N;
        self.len -= 1;
        item
    }

    /// Empties the queue.
    pub fn clear(&mut self) {
        while self.pop().is_some() {}
    }
}

impl<E, const N: usize> Default for Queue<E, N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fifo_and_bounded() {
        let mut q = Queue::<u8, 2>::new();
        assert_eq!((q.free(), q.is_empty()), (2, true));
        assert_eq!(q.push(1), Ok(()));
        assert_eq!(q.push(2), Ok(()));
        assert_eq!(q.push(3), Err(3));
        assert_eq!(q.pop(), Some(1));
        assert_eq!(q.push(4), Ok(()));
        assert_eq!(q.len(), 2);
        assert_eq!((q.pop(), q.pop(), q.pop()), (Some(2), Some(4), None));
        q.push(5).unwrap();
        q.clear();
        assert!(q.is_empty());
    }
}
