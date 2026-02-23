/// RF power meter model definitions, frequency/power lookup tables, and
/// bilinear interpolation for voltage-to-power conversion.
///
/// Each of the 8 supported device models has a fixed set of parameters
/// (frequency count, power count, power range) and an algorithmically
/// generated frequency table that must match the original firmware exactly.

/// Specification for a single RF power meter model.
#[derive(Debug, Clone)]
pub struct ModelSpec {
    pub id: u16,
    pub name: &'static str,
    pub sensor: &'static str,
    pub freq_count: usize,
    pub power_count: usize,
    pub power_min: f32,
    pub power_max: f32,
}

impl ModelSpec {
    /// Look up a model specification by its numeric ID (101..=108).
    /// Returns `None` for unknown IDs.
    pub fn from_id(id: u16) -> Option<Self> {
        let spec = match id {
            101 => ModelSpec {
                id: 101,
                name: "RPM-20GS",
                sensor: "ARW28340",
                freq_count: 202,
                power_count: 21,
                power_min: -40.0,
                power_max: 10.0,
            },
            102 => ModelSpec {
                id: 102,
                name: "RPM-3GS",
                sensor: "AD8362",
                freq_count: 33,
                power_count: 25,
                power_min: -50.0,
                power_max: 10.0,
            },
            103 => ModelSpec {
                id: 103,
                name: "RPM-9G",
                sensor: "ARW22347",
                freq_count: 92,
                power_count: 21,
                power_min: -40.0,
                power_max: 10.0,
            },
            104 => ModelSpec {
                id: 104,
                name: "RPM-20GS v2",
                sensor: "ARW28340",
                freq_count: 218,
                power_count: 21,
                power_min: -40.0,
                power_max: 10.0,
            },
            105 => ModelSpec {
                id: 105,
                name: "RPM-3GS v2",
                sensor: "AD8362",
                freq_count: 58,
                power_count: 25,
                power_min: -50.0,
                power_max: 10.0,
            },
            106 => ModelSpec {
                id: 106,
                name: "RPM-9G v2",
                sensor: "ARW22347",
                freq_count: 99,
                power_count: 21,
                power_min: -40.0,
                power_max: 10.0,
            },
            107 => ModelSpec {
                id: 107,
                name: "RPM-6GH",
                sensor: "ARW22283",
                freq_count: 69,
                power_count: 41,
                power_min: -80.0,
                power_max: 20.0,
            },
            108 => ModelSpec {
                id: 108,
                name: "RPM-20GS OPT260",
                sensor: "ARW28340",
                freq_count: 283,
                power_count: 21,
                power_min: -40.0,
                power_max: 10.0,
            },
            _ => return None,
        };
        Some(spec)
    }

    /// Build the frequency table for this model. All values are in Hz.
    /// The generated table must match the original firmware exactly.
    pub fn build_freq_table(&self) -> Vec<i64> {
        match self.id {
            101 => build_freq_table_101(),
            102 => build_freq_table_102(),
            103 => build_freq_table_103(),
            104 => build_freq_table_104(),
            105 => build_freq_table_105(),
            106 => build_freq_table_106(),
            107 => build_freq_table_107(),
            108 => build_freq_table_108(),
            _ => Vec::new(),
        }
    }

    /// Build the power table for this model (dBm values).
    /// All models use: `power_min + index * 2.5`
    pub fn build_power_table(&self) -> Vec<f32> {
        (0..self.power_count)
            .map(|i| self.power_min + i as f32 * 2.5)
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Frequency table builders (one per model)
// ---------------------------------------------------------------------------

fn build_freq_table_101() -> Vec<i64> {
    let mut t = vec![0i64; 202];
    t[0] = 10_000_000;
    t[1] = 50_000_000;
    for k in 2..202 {
        t[k] = 100_000_000 * (k as i64 - 1);
    }
    t
}

fn build_freq_table_102() -> Vec<i64> {
    let mut t = vec![0i64; 33];
    t[0] = 50;
    t[1] = 10_000_000;
    t[2] = 50_000_000;
    for k in 3..33 {
        t[k] = 100_000_000 * (k as i64 - 2);
    }
    t
}

fn build_freq_table_103() -> Vec<i64> {
    let mut t = vec![0i64; 92];
    t[0] = 10_000_000;
    t[1] = 50_000_000;
    for k in 2..92 {
        t[k] = 100_000_000 * (k as i64 - 1);
    }
    t
}

fn build_freq_table_104() -> Vec<i64> {
    let mut t = vec![0i64; 218];
    for k in 0..18 {
        t[k] = 10_000_000 + 5_000_000 * k as i64;
    }
    for l in 18..218 {
        t[l] = 100_000_000 * (l as i64 - 17);
    }
    t
}

/// Model 105 frequency table, derived from the original .NET source
/// (lookup_table_105_AD8362.cs):
///
///   table_freq[0]      = 50 Hz
///   k in 1..10:  table_freq[k] = 100_000 * k       (100 kHz .. 900 kHz)
///   l in 10..19: table_freq[l] = 1_000_000 * (l-9)  (1 MHz .. 9 MHz)
///   m in 19..28: table_freq[m] = 10_000_000 * (m-18) (10 MHz .. 90 MHz)
///   n in 28..58: table_freq[n] = 100_000_000 * (n-27) (100 MHz .. 3000 MHz)
fn build_freq_table_105() -> Vec<i64> {
    let mut t = vec![0i64; 58];
    t[0] = 50;
    for k in 1..10 {
        t[k] = 100_000 * k as i64;
    }
    for l in 10..19 {
        t[l] = 1_000_000 * (l as i64 - 9);
    }
    for m in 19..28 {
        t[m] = 10_000_000 * (m as i64 - 18);
    }
    for n in 28..58 {
        t[n] = 100_000_000 * (n as i64 - 27);
    }
    t
}

fn build_freq_table_106() -> Vec<i64> {
    let mut t = vec![0i64; 99];
    for k in 0..9 {
        t[k] = 10_000_000 + 10_000_000 * k as i64;
    }
    for l in 9..99 {
        t[l] = 100_000_000 * (l as i64 - 8);
    }
    t
}

fn build_freq_table_107() -> Vec<i64> {
    let mut t = vec![0i64; 69];
    for k in 0..9 {
        t[k] = 10_000_000 + 10_000_000 * k as i64;
    }
    for l in 9..69 {
        t[l] = 100_000_000 * (l as i64 - 8);
    }
    t
}

fn build_freq_table_108() -> Vec<i64> {
    let mut t = vec![0i64; 283];
    for k in 0..18 {
        t[k] = 10_000_000 + 5_000_000 * k as i64;
    }
    for l in 18..283 {
        t[l] = 100_000_000 * (l as i64 - 17);
    }
    t
}

// ---------------------------------------------------------------------------
// ADC to voltage
// ---------------------------------------------------------------------------

/// Convert a raw 24-bit ADC reading to a voltage (mV).
/// The ADC full-scale value 8388607 (2^23 - 1) maps to 1500 mV.
pub fn adc_to_voltage(adc: u32) -> f32 {
    (adc as f64 * 1500.0 / 8388607.0) as f32
}

// ---------------------------------------------------------------------------
// 2D bilinear interpolation
// ---------------------------------------------------------------------------

/// Convert a voltage reading at a given frequency to power in dBm using
/// bilinear interpolation across the frequency and power axes of the
/// calibration table.
///
/// Returns `None` for out-of-range inputs (frequency outside table bounds,
/// or voltage above the maximum calibrated value at either bracketing
/// frequency).
pub fn interpolate_power(
    freq: i64,
    voltage: f32,
    freq_table: &[i64],
    power_table: &[f32],
    voltage_table: &[Vec<f32>],
    power_min: f32,
) -> Option<f32> {
    let freq_count = freq_table.len();
    let power_count = power_table.len();

    // Step 1: Boundary check on frequency
    if freq < freq_table[0] || freq > freq_table[freq_count - 1] {
        return None;
    }

    // Step 2: Find bracketing frequency indices
    let mut idx1 = 0usize;
    let mut idx2 = 0usize;
    for i in 0..freq_count - 1 {
        if freq >= freq_table[i] && freq <= freq_table[i + 1] {
            idx1 = i;
            idx2 = i + 1;
            break;
        }
    }

    // Step 3: Check if voltage exceeds max at either frequency
    if voltage >= voltage_table[idx1][power_count - 1]
        || voltage >= voltage_table[idx2][power_count - 1]
    {
        return None; // Over-range
    }

    let mut power_out1: f32;
    let mut power_out2: f32;

    // Step 4: Handle voltage below minimum
    if voltage < voltage_table[idx1][0] || voltage < voltage_table[idx2][0] {
        power_out1 = power_table[0] + voltage * (power_table[0] - 3000.0);
        power_out2 = power_table[0] + voltage * (power_table[0] - 3000.0);
    } else {
        // Step 5: Interpolate at frequency idx1
        power_out1 = 100000.0; // sentinel
        for j in 0..power_count - 1 {
            if voltage >= voltage_table[idx1][j] && voltage <= voltage_table[idx1][j + 1] {
                power_out1 = power_table[j]
                    + (voltage - voltage_table[idx1][j])
                        / (voltage_table[idx1][j + 1] - voltage_table[idx1][j])
                        * (power_table[j + 1] - power_table[j]);
                break;
            }
        }

        // Step 6: Interpolate at frequency idx2
        power_out2 = 100000.0; // sentinel
        for k in 0..power_count - 1 {
            if voltage >= voltage_table[idx2][k] && voltage <= voltage_table[idx2][k + 1] {
                power_out2 = power_table[k]
                    + (voltage - voltage_table[idx2][k])
                        / (voltage_table[idx2][k + 1] - voltage_table[idx2][k])
                        * (power_table[k + 1] - power_table[k]);
                break;
            }
        }
    }

    // Step 7: Clamp to min
    if power_out1 < power_min {
        power_out1 = power_min - (power_out1 - power_out1.floor());
    }
    if power_out2 < power_min {
        power_out2 = power_min - (power_out2 - power_out2.floor());
    }

    // Step 8: Interpolate between the two frequency points
    let proportion = ((freq - freq_table[idx1]) as f64
        / (freq_table[idx2] - freq_table[idx1]) as f64)
        .min(1.0);

    Some((power_out1 as f64 * (1.0 - proportion) + power_out2 as f64 * proportion) as f32)
}

// ---------------------------------------------------------------------------
// DeviceCalibration: runtime container for a device's calibration data
// ---------------------------------------------------------------------------

/// Holds the complete calibration state for a connected device: the model
/// specification, pre-computed frequency and power tables, and the
/// voltage calibration matrix loaded from the device at runtime.
pub struct DeviceCalibration {
    pub spec: ModelSpec,
    pub freq_table: Vec<i64>,
    pub power_table: Vec<f32>,
    pub voltage_table: Vec<Vec<f32>>,
}

impl DeviceCalibration {
    /// Create a new calibration container for the given model ID.
    /// The voltage table is initialized to all zeros; call
    /// `set_calibration_row` to populate it with data read from the device.
    pub fn new(model_id: u16) -> Option<Self> {
        let spec = ModelSpec::from_id(model_id)?;
        let freq_table = spec.build_freq_table();
        let power_table = spec.build_power_table();
        let voltage_table = vec![vec![0.0f32; spec.power_count]; spec.freq_count];
        Some(Self {
            spec,
            freq_table,
            power_table,
            voltage_table,
        })
    }

    /// Store a row of voltage calibration data for the given frequency index.
    pub fn set_calibration_row(&mut self, freq_index: usize, voltages: Vec<f32>) {
        if freq_index < self.voltage_table.len() {
            self.voltage_table[freq_index] = voltages;
        }
    }

    /// Convert a voltage reading at a specific frequency to power in dBm.
    pub fn convert(&self, freq_hz: i64, voltage: f32) -> Option<f32> {
        interpolate_power(
            freq_hz,
            voltage,
            &self.freq_table,
            &self.power_table,
            &self.voltage_table,
            self.spec.power_min,
        )
    }
}
