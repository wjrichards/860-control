//! Editing one mouse profile's report rate and DPI steps in the app, before
//! it is sent to the service. No GTK here, so the rules are unit-tested.

/// Report periods in ms with their labels, fastest first.
pub const REPORT_RATES: [(u8, &str); 4] = [(1, "1000 Hz"), (2, "500 Hz"), (4, "250 Hz"), (8, "125 Hz")];
pub const MAX_STEPS: usize = 5;

/// The sensor's DPI range: min to max in steps of `step`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DpiRange {
    pub min: u16,
    pub max: u16,
    pub step: u16,
}

impl DpiRange {
    /// `dpi` moved onto the range and its step grid.
    pub fn snap(&self, dpi: u16) -> u16 {
        let step = self.step.max(1);
        let clamped = dpi.clamp(self.min, self.max);
        let snapped = self.min + (clamped - self.min + step / 2) / step * step;
        snapped.min(self.max)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileEdit {
    pub report_rate_ms: u8,
    pub default_index: u8,
    pub dpis: Vec<u16>,
}

impl ProfileEdit {
    /// Add a step after the last: double it, kept within the range.
    /// Returns whether one was added (at most [`MAX_STEPS`]).
    pub fn add_step(&mut self, range: DpiRange) -> bool {
        if self.dpis.len() >= MAX_STEPS {
            return false;
        }
        let last = self.dpis.last().copied().unwrap_or(range.min);
        self.dpis.push(range.snap(last.saturating_mul(2)));
        true
    }

    /// Remove step `i`, keeping at least one; the default step follows the
    /// step it pointed at, or the one before it if that was removed.
    pub fn remove_step(&mut self, i: usize) -> bool {
        if self.dpis.len() <= 1 || i >= self.dpis.len() {
            return false;
        }
        self.dpis.remove(i);
        let d = usize::from(self.default_index);
        if d >= i {
            // A later default moves down with its step; a removed one falls
            // back to the step before it.
            let d = d.saturating_sub(1).min(self.dpis.len() - 1);
            self.default_index = u8::try_from(d).unwrap_or(0);
        }
        true
    }

    pub fn set_default(&mut self, i: usize) {
        if i < self.dpis.len() {
            self.default_index = u8::try_from(i).unwrap_or(0);
        }
    }

    pub fn set_dpi(&mut self, i: usize, dpi: u16, range: DpiRange) {
        if let Some(d) = self.dpis.get_mut(i) {
            *d = range.snap(dpi);
        }
    }

    /// The index in [`REPORT_RATES`] of this profile's rate (1000 Hz if unknown).
    pub fn rate_position(&self) -> usize {
        REPORT_RATES.iter().position(|&(ms, _)| ms == self.report_rate_ms).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RANGE: DpiRange = DpiRange { min: 100, max: 25600, step: 50 };

    fn edit(default_index: u8, dpis: &[u16]) -> ProfileEdit {
        ProfileEdit { report_rate_ms: 1, default_index, dpis: dpis.to_vec() }
    }

    #[test]
    fn snap_to_the_grid() {
        assert_eq!([RANGE.snap(1610), RANGE.snap(1630), RANGE.snap(40), RANGE.snap(30000)], [1600, 1650, 100, 25600]);
    }

    #[test]
    fn add_doubles_up_to_five() {
        let mut e = edit(0, &[800]);
        assert!(e.add_step(RANGE) && e.add_step(RANGE));
        assert_eq!(e.dpis, [800, 1600, 3200]);
        let mut big = edit(0, &[16000]);
        big.add_step(RANGE);
        assert_eq!(big.dpis, [16000, 25600], "kept within the range");
        let mut full = edit(0, &[100, 200, 300, 400, 500]);
        assert!(!full.add_step(RANGE));
        assert_eq!(full.dpis.len(), 5);
    }

    #[test]
    fn remove_keeps_one_and_follows_the_default() {
        let mut e = edit(2, &[400, 800, 1600, 3200]);
        assert!(e.remove_step(0));
        assert_eq!((e.default_index, e.dpis.clone()), (1, vec![800, 1600, 3200]), "still 1600");
        assert!(e.remove_step(1));
        assert_eq!((e.default_index, e.dpis.clone()), (0, vec![800, 3200]), "the removed default falls back");
        assert!(e.remove_step(1));
        assert!(!e.remove_step(0), "the last step stays");
        assert_eq!(e.dpis, [800]);
    }

    #[test]
    fn set_default_and_dpi() {
        let mut e = edit(0, &[800, 1600]);
        e.set_default(1);
        e.set_default(5);
        assert_eq!(e.default_index, 1);
        e.set_dpi(0, 1234, RANGE);
        assert_eq!(e.dpis, [1250, 1600]);
    }

    #[test]
    fn report_rate_positions() {
        let positions: Vec<usize> =
            [1, 2, 4, 8, 3].iter().map(|&ms| ProfileEdit { report_rate_ms: ms, ..edit(0, &[800]) }.rate_position()).collect();
        assert_eq!(positions, [0, 1, 2, 3, 0]);
    }
}
