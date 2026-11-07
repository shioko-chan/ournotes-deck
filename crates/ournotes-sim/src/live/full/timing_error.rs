//! Deterministic timing-error play. Greats are supplied by the accuracy stream. For each
//! remaining Just-eligible note at its judge time, accumulate erf(w / (sigma * sqrt(2))).
//! Emit Just whenever the accumulated mass crosses an integer; otherwise emit Perfect.
//! The accumulator spans the live, in stream order, and is independent of random seeds.
//! This is a deterministic realization, not an expectation over random judgement streams.
use super::{GekisouSetup, Master, RawJudgementRuntime};
use crate::{
    Error,
    live::{raw::TimingUnit, raw_windows::TimingSet},
};
use std::collections::BTreeMap;

/// A finite positive standard deviation, stored as bits to preserve exact configuration identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimingError(u64);
impl TimingError {
    pub fn new(sigma_ms: f64) -> Result<Self, Error> {
        if !sigma_ms.is_finite() || sigma_ms <= 0.0 {
            return Err(Error::Input("sigmaMs must be finite and positive".into()));
        }
        Ok(Self(sigma_ms.to_bits()))
    }
    pub fn sigma_ms(self) -> f64 {
        f64::from_bits(self.0)
    }
}

// Abramowitz–Stegun 7.1.26; maximum absolute error approximately 1.5e-7.
// Clamp to a probability and preserve the exact zero boundary.
fn chance(window_ms: i32, sigma: f64) -> f64 {
    if window_ms <= 0 {
        return 0.0;
    }
    let x = f64::from(window_ms) / sigma / std::f64::consts::SQRT_2;
    let t = 1.0 / (1.0 + 0.3275911 * x);
    (1.0 - (((((1.061405429 * t - 1.453152027) * t) + 1.421413741) * t - 0.284496736) * t + 0.254829592)
        * t
        * (-x * x).exp())
    .clamp(0.0, 1.0)
}

#[derive(Clone, Debug)]
pub(super) struct Runtime {
    pub(super) windows: RawJudgementRuntime,
    config: TimingError,
    ranges: Vec<(i32, i32)>,
    mass: f64,
}
impl Runtime {
    pub(super) fn new(master: &Master, setup: &GekisouSetup, config: TimingError) -> Result<Self, Error> {
        let mut rows: Vec<_> = master.live_judgement_timings.iter().filter(|r| r.assist_level == 0).collect();
        rows.sort_by_key(|r| (r.note_judgement_type, r.judgement_priority));
        let mut sets: BTreeMap<i32, Vec<TimingUnit>> = BTreeMap::new();
        let integer = |v| i32::try_from(v).map_err(|_| Error::Master("timing value exceeds i32".into()));
        for r in rows {
            if r.before_ms < 0 || r.after_ms < 0 {
                return Err(Error::Master("negative timing window".into()));
            }
            sets.entry(integer(r.note_judgement_type)?).or_default().push(TimingUnit::new(
                integer(r.note_simulate_judgement)?,
                integer(r.before_ms)?,
                integer(r.after_ms)?,
            ));
        }
        if sets.is_empty() {
            return Err(Error::Master("timingError requires judgement timing rows".into()));
        }
        Ok(Self {
            windows: RawJudgementRuntime::new(
                sets.into_iter().map(|(judgement_type, units)| TimingSet { judgement_type, units }).collect(),
                0,
                0,
            ),
            config,
            ranges: setup.fevers.iter().zip(&setup.missions).filter_map(|(&r, &m)| (m == 3).then_some(r)).collect(),
            mass: 0.0,
        })
    }
    pub(super) fn judgement(&mut self, judgement_type: i32, time: i32, original: i32) -> i32 {
        if original != 5 || !self.ranges.iter().any(|&(a, b)| a <= time && time < b) {
            return original;
        }
        let Some(window) = self
            .windows
            .effective_timings()
            .iter()
            .find(|s| s.judgement_type == judgement_type)
            .and_then(|s| s.units.iter().find(|u| u.judgement == 6 && u.enabled))
        else {
            return original;
        };
        self.mass += chance(window.before_ms, self.config.sigma_ms());
        if self.mass >= 1.0 {
            self.mass -= 1.0;
            6
        } else {
            5
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn probability_and_validation() {
        assert_eq!(chance(0, 10.0), 0.0);
        assert!((chance(10, 10.0) - 0.682689492).abs() < 2e-7);
        assert!(chance(20, 10.0) > chance(10, 10.0));
        for sigma in [0.0, -1.0, f64::INFINITY, f64::NAN] {
            assert!(TimingError::new(sigma).is_err());
        }
    }
    #[test]
    fn active_4004_windows_stack_cap_and_revert_by_owner() {
        use super::super::{
            EffectRow, StateKey,
            applier_plan::ApplierPlan,
            engine::{END_FRAME, EXECUTE_FRAME},
        };
        let mut windows = RawJudgementRuntime::new(
            vec![TimingSet { judgement_type: 1, units: vec![TimingUnit::new(6, 10, 10), TimingUnit::new(1, 25, 25)] }],
            0,
            0,
        );
        let row = EffectRow {
            id: 1,
            effect_type: 4004,
            effect_value: 10000,
            max_effect_value: 0,
            effect_limit_count: 0,
            targets: Ok(vec![6]),
            applier: ApplierPlan::compile(4004, true),
        };
        let a = StateKey::Cond { effect_id: 1, index: 0 };
        let b = StateKey::Cond { effect_id: 2, index: 0 };
        windows.update_effect(a, EXECUTE_FRAME, &row).unwrap();
        assert_eq!(windows.effective_timings()[0].units[0].before_ms, 20);
        windows.update_effect(b, EXECUTE_FRAME, &row).unwrap();
        assert_eq!(windows.effective_timings()[0].units[0].before_ms, 25);
        windows.update_effect(a, END_FRAME, &row).unwrap();
        assert_eq!(windows.effective_timings()[0].units[0].before_ms, 15);
        windows.update_effect(b, END_FRAME, &row).unwrap();
        assert_eq!(windows.effective_timings()[0].units[0].before_ms, 10);
    }
    #[test]
    fn dither_counts_mass_and_skips_greats_and_outside_ranges() {
        let mut runtime = Runtime {
            windows: RawJudgementRuntime::new(
                vec![TimingSet { judgement_type: 1, units: vec![TimingUnit::new(6, 10, 10)] }],
                0,
                0,
            ),
            config: TimingError::new(10.0).unwrap(),
            ranges: vec![(0, 100)],
            mass: 0.0,
        };
        assert_eq!(runtime.judgement(1, -1, 5), 5);
        assert_eq!(runtime.judgement(1, 0, 4), 4);
        assert_eq!(runtime.judgement(2, 0, 5), 5);
        let count = (0..100).filter(|&t| runtime.judgement(1, t, 5) == 6).count();
        assert_eq!(count, 68);
        assert_eq!(runtime.judgement(1, 100, 5), 5);
    }
}
