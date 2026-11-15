can-common
==========

What CAN controller drivers share, whatever traits they implement (can-hal's,
embedded-can's) and whatever controller they drive: `no_std`, no
dependencies, and none of any CAN trait crate's types, so a driver maps them
to the traits it implements.

- `ordering::Outstanding`: transmissions in the controller's transmit
  buffers, reported in the order they were handed over whatever order they
  finish in (`Done::Sent`, `Aborted` or `GaveUp`), as a confirmed-transmission
  channel needs.
- `queue::Queue`: a bounded FIFO with a `const` constructor, for the events
  an interrupt handler leaves for the channel.
- `timing::Limits::solve`: the bit-timing solver, for any controller's field
  ranges: a bit rate and sample point to `Segments` (prescaler, seg1, seg2,
  jump width), preferring the most quanta.
- `Clock`: the source of timestamps and timeouts a driver is given.
- `Warning` and `Counts`: the error-warning level with a waiter, and running
  counts for statistics.

Its users: pic32c-hal's `can` (Microchip's M_CAN), and can-hal-embedded's
`can-hal-bxcan` and `can-hal-fdcan` (ST), each of which had its own copy.
Private for now, like the drivers that use it.

# Licence

MIT OR Apache-2.0, at your option.
