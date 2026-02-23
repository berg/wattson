// src/state.rs
use std::collections::VecDeque;

pub struct RingBuffer {
    data: VecDeque<f64>,
    capacity: usize,
}

impl RingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, value: f64) {
        if self.data.len() == self.capacity {
            self.data.pop_front();
        }
        self.data.push_back(value);
    }

    pub fn iter(&self) -> impl Iterator<Item = &f64> {
        self.data.iter()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

pub struct Stats {
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub count: u64,
    sum: f64,
}

impl Stats {
    pub fn new() -> Self {
        Self {
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
            avg: 0.0,
            count: 0,
            sum: 0.0,
        }
    }

    pub fn update(&mut self, value: f64) {
        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }
        self.count += 1;
        self.sum += value;
        self.avg = self.sum / self.count as f64;
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

/// Convert dBm to linear watts, returning (value, unit_prefix)
/// Auto-selects best prefix: W, mW, uW, nW
pub fn format_linear_power(dbm: f64) -> (f64, &'static str) {
    let watts = 10.0f64.powf((dbm - 30.0) / 10.0);
    if watts >= 1.0 {
        (watts, "W")
    } else if watts >= 1e-3 {
        (watts * 1e3, "mW")
    } else if watts >= 1e-6 {
        (watts * 1e6, "uW")
    } else {
        (watts * 1e9, "nW")
    }
}

/// Full application state
pub struct AppState {
    pub current_dbm: Option<f64>,
    pub stats: Stats,
    pub waveform: RingBuffer,
    pub frequency_mhz: f64,
    pub offset_db: f64,
    pub sampling_rate: crate::protocol::SamplingRate,
    pub logging: bool,
    pub paused: bool,
    pub auto_scale: bool,
    pub y_min: f64,
    pub y_max: f64,
    pub show_help: bool,
    // Device info
    pub model_name: Option<String>,
    pub serial_number: Option<u32>,
    pub firmware_version: Option<String>,
    // Connection state
    pub connected: bool,
    pub calibration_progress: Option<(usize, usize)>,
    /// When Some, shows a text input prompt in the status bar: (label, buffer)
    pub input_prompt: Option<(String, String)>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            current_dbm: None,
            stats: Stats::new(),
            waveform: RingBuffer::new(76_800),
            frequency_mhz: 10.0,
            offset_db: 0.0,
            sampling_rate: crate::protocol::SamplingRate::Hz640,
            logging: false,
            paused: false,
            auto_scale: true,
            y_min: -50.0,
            y_max: 10.0,
            show_help: false,
            model_name: None,
            serial_number: None,
            firmware_version: None,
            connected: false,
            calibration_progress: None,
            input_prompt: None,
        }
    }

    pub fn push_reading(&mut self, dbm: f64) {
        if dbm.is_nan() {
            self.current_dbm = None; // Signal over-range
            return;
        }
        let adjusted = dbm + self.offset_db;
        self.current_dbm = Some(adjusted);
        if !self.paused {
            self.stats.update(adjusted);
            self.waveform.push(adjusted);
        }
    }
}
