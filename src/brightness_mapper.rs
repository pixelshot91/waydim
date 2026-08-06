use crate::common::Nit;

/// Represents a calibration measurement point from a photometer.
#[derive(Debug, Clone, Copy)]
pub struct SamplePoint {
    pub sw: f64,
    pub hw: u32,
    pub nits: f64,
}

/// Output settings for display brightness control.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrightnessSetting {
    pub sw: f64,
    pub hw: u32,
}

/// A photometrically calibrated brightness mapper.
pub struct BrightnessMapper {
    /// Calibration samples for hw > 1 (sw = 1.0), sorted by nits ascending.
    hw_samples: Vec<SamplePoint>,
    /// Luminance in nits at hw = 1, sw = 1.0 (typically 2.0 nits given 0.5 sw = 1.0 nit).
    min_hw_nits: f64,
    /// Maximum achievable luminance at max hw and sw = 1.0.
    max_nits: f64,
}

impl BrightnessMapper {
    /// Constructs a mapper given a series of photometer readings.
    ///
    /// # Panics
    /// Panics if sample points are empty or non-monotonic.
    pub fn new(mut samples: Vec<SamplePoint>) -> Self {
        assert!(!samples.is_empty(), "Calibration samples cannot be empty.");

        // Sort samples by luminance ascending
        samples.sort_by(|a, b| a.nits.partial_cmp(&b.nits).unwrap());

        let min_hw_nits = samples
            .iter()
            .find(|s| s.hw == 1 && (s.sw - 1.0).abs() < f64::EPSILON)
            .map(|s| s.nits)
            .unwrap_or(2.0); // Derived from sw=0.5 -> 1.0 nit invariant

        let max_nits = samples.last().unwrap().nits;

        Self {
            hw_samples: samples,
            min_hw_nits,
            max_nits,
        }
    }

    /// Maps target physical luminance (in nits) to software and hardware values.
    ///
    /// - For `target_nits >= min_hw_nits`: `sw = 1.0`, `hw` is interpolated.
    /// - For `target_nits < min_hw_nits`: `hw = 1`, `sw = target_nits / min_hw_nits`.
    pub fn nits_to_setting(&self, target_nits: Nit) -> BrightnessSetting {
        if target_nits.0 < self.min_hw_nits {
            // Software dimming region: hw strictly fixed to 1
            let sw_value = (target_nits.0 / self.min_hw_nits).clamp(0.0, 1.0);
            BrightnessSetting {
                sw: sw_value,
                hw: 1,
            }
        } else if target_nits.0 >= self.max_nits {
            // Upper bound clamp
            let max_hw = self.hw_samples.last().unwrap().hw;
            BrightnessSetting {
                sw: 1.0,
                hw: max_hw,
            }
        } else {
            // Hardware dimming region: sw strictly fixed to 1.0
            // Find bounding calibration segment for piecewise linear interpolation
            let i = self
                .hw_samples
                .windows(2)
                .position(|w| w[0].nits <= target_nits.0 && target_nits.0 <= w[1].nits)
                .unwrap_or(0);

            let p0 = &self.hw_samples[i];
            let p1 = &self.hw_samples[i + 1];

            // Linear interpolation in log-space or linear-space for hw register
            let t = (target_nits.0 - p0.nits) / (p1.nits - p0.nits);
            let hw_interpolated = p0.hw as f64 + t * (p1.hw as f64 - p0.hw as f64);

            BrightnessSetting {
                sw: 1.0,
                hw: hw_interpolated.round() as u32,
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::brightness_mapper::*;

    #[test]
    fn test() {
        // Empirical photometer calibration dataset
        let calibration = vec![
            SamplePoint {
                sw: 0.5,
                hw: 1,
                nits: 1.0,
            },
            SamplePoint {
                sw: 1.0,
                hw: 1,
                nits: 2.0,
            }, // Derived min hardware point
            SamplePoint {
                sw: 1.0,
                hw: 10,
                nits: 8.5,
            },
            SamplePoint {
                sw: 1.0,
                hw: 100,
                nits: 45.0,
            },
            SamplePoint {
                sw: 1.0,
                hw: 1000,
                nits: 180.0,
            },
            SamplePoint {
                sw: 1.0,
                hw: 10000,
                nits: 400.0,
            },
        ];

        let mapper = BrightnessMapper::new(calibration);

        // Test Case 1: Sub-hardware threshold (0.5 nits -> hw=1, sw=0.25)
        let s1 = mapper.nits_to_setting(Nit::try_new(0.5).unwrap());
        println!("0.5 nits  -> {:?}", s1);
        assert_eq!(s1.hw, 1);
        assert!((s1.sw - 0.25).abs() < 1e-6);

        // Test Case 2: Standard hardware region (100 nits -> hw= interpolated, sw=1.0)
        let s2 = mapper.nits_to_setting(Nit::try_new(100.0).unwrap());
        println!("100 nits  -> {:?}", s2);
        assert_eq!(s2.sw, 1.0);
        assert!(s2.hw > 100 && s2.hw < 1000);
    }
}
