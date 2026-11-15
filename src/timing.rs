// SPDX-FileCopyrightText: 2026-2026. Aeolionics, LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Bit timing: a bit as time quanta, one of sync, then `seg1` before the
//! sample point and `seg2` after it, each quantum `prescaler` cycles of the
//! controller's clock; and the solver that picks them for a bit rate and
//! sample point within a controller's field ranges.

/// A bit's timing in time quanta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Segments {
    /// Cycles of the controller's clock per quantum.
    pub prescaler: u16,
    /// Quanta from the end of sync to the sample point: propagation and
    /// phase 1 together.
    pub seg1: u16,
    /// Quanta from the sample point to the end of the bit: phase 2.
    pub seg2: u16,
    /// The synchronisation jump width, in quanta.
    pub sjw: u16,
}

impl Segments {
    /// Quanta in a bit.
    #[must_use]
    pub const fn quanta(self) -> u32 {
        1 + self.seg1 as u32 + self.seg2 as u32
    }

    /// The bit rate from a `clock_hz` clock, 0 if the prescaler is 0.
    #[must_use]
    pub const fn bitrate(self, clock_hz: u32) -> u32 {
        let cycles = self.prescaler as u32 * self.quanta();
        match clock_hz.checked_div(cycles) {
            Some(rate) => rate,
            None => 0,
        }
    }

    /// The sample point, in tenths of a percent of the bit.
    #[must_use]
    pub const fn sample_point_per_mille(self) -> u32 {
        (1 + self.seg1 as u32) * 1000 / self.quanta()
    }
}

/// The largest value of each field a controller (or one of its phases)
/// allows; each is at least 1, and the jump width at most `seg2`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Limits {
    /// The largest prescaler.
    pub prescaler: u16,
    /// The largest `seg1`.
    pub seg1: u16,
    /// The largest `seg2`.
    pub seg2: u16,
    /// The largest jump width.
    pub sjw: u16,
}

impl Limits {
    /// `segments`, if every field is in range.
    #[must_use]
    pub const fn check(&self, segments: Segments) -> Option<Segments> {
        let Segments { prescaler, seg1, seg2, sjw } = segments;
        if prescaler < 1 || prescaler > self.prescaler || seg1 < 1 || seg1 > self.seg1 || seg2 < 1 || seg2 > self.seg2 || sjw < 1 || sjw > self.sjw || sjw > seg2 {
            return None;
        }
        Some(segments)
    }

    /// The timing for `bitrate` from a `clock_hz` clock whose sample point is
    /// nearest `sample_point_per_mille` (875 for 87.5 %), with as many quanta
    /// as allow it (the smallest prescaler, as `CiA` 601-3 advises, and the
    /// finest resynchronisation), and the widest jump width the limits and
    /// `seg2` allow. `None` when no whole number of quanta divides the clock
    /// into `bitrate` within the limits.
    #[must_use]
    pub fn solve(&self, clock_hz: u32, bitrate: u32, sample_point_per_mille: u16) -> Option<Segments> {
        let target = u32::from(sample_point_per_mille);
        let most = 1 + u32::from(self.seg1) + u32::from(self.seg2);
        let mut best: Option<(u32, Segments)> = None;
        for quanta in (3..=most).rev() {
            let Some(cycles) = bitrate.checked_mul(quanta) else { continue };
            if cycles == 0 || !clock_hz.is_multiple_of(cycles) {
                continue;
            }
            let Ok(prescaler) = u16::try_from(clock_hz / cycles) else { continue };
            // The sample point is at the end of seg1, `1 + seg1` quanta in.
            let sample = (quanta * target + 500) / 1000;
            let (Ok(seg1), Ok(seg2)) = (u16::try_from(sample.saturating_sub(1)), u16::try_from(quanta.saturating_sub(sample))) else {
                continue;
            };
            let Some(segments) = self.check(Segments {
                prescaler,
                seg1,
                seg2,
                sjw: seg2.min(self.sjw),
            }) else {
                continue;
            };
            let error = (sample * 1000 / quanta).abs_diff(target);
            if best.is_none_or(|(best_error, _)| error < best_error) {
                best = Some((error, segments));
            }
        }
        best.map(|(_, segments)| segments)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bosch `M_CAN`'s nominal phase (NBTP) and data phase (DBTP).
    const NOMINAL: Limits = Limits {
        prescaler: 512,
        seg1: 256,
        seg2: 128,
        sjw: 128,
    };
    const DATA: Limits = Limits { prescaler: 32, seg1: 32, seg2: 16, sjw: 16 };
    /// bxCAN's BTR.
    const BXCAN: Limits = Limits { prescaler: 1024, seg1: 16, seg2: 8, sjw: 4 };

    #[test]
    fn nominal_rates_from_80_mhz() {
        for bitrate in [125_000, 250_000, 500_000, 1_000_000] {
            let s = NOMINAL.solve(80_000_000, bitrate, 875).unwrap();
            assert_eq!((s.bitrate(80_000_000), s.sample_point_per_mille()), (bitrate, 875), "{bitrate}");
        }
        // 500 kbit/s: 160 quanta of one cycle each.
        assert_eq!(NOMINAL.solve(80_000_000, 500_000, 875), Some(Segments { prescaler: 1, seg1: 139, seg2: 20, sjw: 20 }));
    }

    #[test]
    fn data_rates_from_80_mhz() {
        for bitrate in [1_000_000, 2_000_000, 4_000_000, 5_000_000, 8_000_000] {
            assert_eq!(DATA.solve(80_000_000, bitrate, 800).unwrap().bitrate(80_000_000), bitrate, "{bitrate}");
        }
        assert_eq!(DATA.solve(80_000_000, 2_000_000, 800), Some(Segments { prescaler: 1, seg1: 31, seg2: 8, sjw: 8 }));
    }

    /// NMEA 2000's 250 kbit/s at 87.5 %, from clocks a PIC32CK or an STM32
    /// can give its controller.
    #[test]
    fn nmea_2000_rate() {
        for clock_hz in [8_000_000, 36_000_000, 40_000_000, 48_000_000, 60_000_000, 80_000_000] {
            for limits in [NOMINAL, BXCAN] {
                let s = limits.solve(clock_hz, 250_000, 875).unwrap();
                assert_eq!((s.bitrate(clock_hz), s.sample_point_per_mille()), (250_000, 875), "{clock_hz}");
            }
        }
        // bxCAN at 48 MHz: 16 quanta of 12 cycles, the jump width capped at 4.
        assert_eq!(BXCAN.solve(48_000_000, 250_000, 875), Some(Segments { prescaler: 12, seg1: 13, seg2: 2, sjw: 2 }));
    }

    #[test]
    fn no_fit() {
        assert_eq!(NOMINAL.solve(80_000_000, 0, 875), None);
        assert_eq!(BXCAN.solve(1_000_000, 1_000_000, 875), None);
        assert_eq!(DATA.solve(7, 3, 800), None);
    }

    #[test]
    fn checks_fields() {
        let ok = Segments { prescaler: 2, seg1: 13, seg2: 2, sjw: 1 };
        assert_eq!(NOMINAL.check(ok), Some(ok));
        assert_eq!(NOMINAL.check(Segments { prescaler: 0, ..ok }), None);
        assert_eq!(NOMINAL.check(Segments { prescaler: 513, ..ok }), None);
        assert_eq!(NOMINAL.check(Segments { seg1: 257, ..ok }), None);
        assert_eq!(DATA.check(Segments { seg2: 17, ..ok }), None);
        assert_eq!(DATA.check(Segments { sjw: 3, ..ok }), None, "wider than seg2");
    }
}
