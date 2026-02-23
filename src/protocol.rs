// src/protocol.rs

pub const SYNC1: u8 = 0x55;
pub const SYNC2: u8 = 0xAA;

pub const CMD_QUERY_ID: u8 = 0x80;
pub const CMD_SET_CONFIG: u8 = 0x83;
pub const CMD_WRITE_CALIBRATION: u8 = 0x85;
pub const CMD_READ_CALIBRATION: u8 = 0x86;

pub const CMD_RESP_ID: u8 = 0x00;
pub const CMD_RESP_CONFIG: u8 = 0x03;
pub const CMD_RESP_CALIBRATION: u8 = 0x05;
pub const CMD_POWER_DATA: u8 = 0x06;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SamplingRate {
    Hz10 = 0,
    Hz40 = 1,
    Hz640 = 2,
    Hz1280 = 3,
}

impl SamplingRate {
    pub fn from_index(idx: u8) -> Option<Self> {
        match idx {
            0 => Some(Self::Hz10),
            1 => Some(Self::Hz40),
            2 => Some(Self::Hz640),
            3 => Some(Self::Hz1280),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Hz10 => "10 Hz",
            Self::Hz40 => "40 Hz",
            Self::Hz640 => "640 Hz",
            Self::Hz1280 => "1280 Hz",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Hz10 => Self::Hz40,
            Self::Hz40 => Self::Hz640,
            Self::Hz640 => Self::Hz1280,
            Self::Hz1280 => Self::Hz10,
        }
    }
}

pub fn checksum(cmd: u8, len: u8, data: &[u8]) -> u8 {
    let inv_len = !len;
    let mut sum = cmd.wrapping_add(len).wrapping_add(inv_len);
    for &b in data {
        sum = sum.wrapping_add(b);
    }
    sum
}

fn build_frame(cmd: u8, data: &[u8]) -> Vec<u8> {
    let len = data.len() as u8;
    let chk = checksum(cmd, len, data);
    let mut frame = vec![SYNC1, SYNC2, cmd, len, !len];
    frame.extend_from_slice(data);
    frame.push(chk);
    frame
}

pub fn build_query_id() -> Vec<u8> {
    build_frame(CMD_QUERY_ID, &[])
}

pub fn build_set_config(rate: SamplingRate) -> Vec<u8> {
    build_frame(CMD_SET_CONFIG, &[rate as u8])
}

pub fn build_read_calibration(freq_index: u16) -> Vec<u8> {
    let data = [(freq_index >> 8) as u8, (freq_index & 0xFF) as u8];
    build_frame(CMD_READ_CALIBRATION, &data)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    DeviceId {
        model_id: u16,
        firmware_version: u16,
        serial_number: u32,
    },
    ConfigAck {
        rate: SamplingRate,
    },
    CalibrationData {
        freq_index: u16,
        voltages: Vec<f32>,
    },
    PowerData {
        adc_value: u32,
    },
}

pub struct FrameParser {
    buf: Vec<u8>,
}

impl FrameParser {
    pub fn new() -> Self {
        Self {
            buf: Vec::with_capacity(512),
        }
    }

    pub fn feed(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
    }

    pub fn next_frame(&mut self) -> Option<Frame> {
        loop {
            let sync_pos = self
                .buf
                .windows(2)
                .position(|w| w[0] == SYNC1 && w[1] == SYNC2)?;
            if sync_pos > 0 {
                self.buf.drain(..sync_pos);
            }

            if self.buf.len() < 5 {
                return None;
            }

            let cmd = self.buf[2];
            let len = self.buf[3] as usize;
            let inv_len = self.buf[4];

            if inv_len != !(len as u8) {
                self.buf.drain(..2);
                continue;
            }

            let frame_len = 5 + len + 1;
            if self.buf.len() < frame_len {
                return None;
            }

            let data = &self.buf[5..5 + len];
            let expected_chk = checksum(cmd, len as u8, data);
            let actual_chk = self.buf[5 + len];

            if expected_chk != actual_chk {
                self.buf.drain(..2);
                continue;
            }

            let frame = self.parse_payload(cmd, data);
            self.buf.drain(..frame_len);
            if let Some(f) = frame {
                return Some(f);
            }
        }
    }

    fn parse_payload(&self, cmd: u8, data: &[u8]) -> Option<Frame> {
        match cmd {
            CMD_RESP_ID if data.len() == 8 => {
                let model_id = u16::from_be_bytes([data[0], data[1]]);
                let firmware_version = u16::from_be_bytes([data[2], data[3]]);
                let serial_number =
                    u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
                Some(Frame::DeviceId {
                    model_id,
                    firmware_version,
                    serial_number,
                })
            }
            CMD_RESP_CONFIG if data.len() == 1 => {
                let rate = SamplingRate::from_index(data[0])?;
                Some(Frame::ConfigAck { rate })
            }
            CMD_RESP_CALIBRATION if data.len() >= 6 => {
                let freq_index = u16::from_be_bytes([data[0], data[1]]);
                let num_voltages = (data.len() - 2) / 4;
                let mut voltages = Vec::with_capacity(num_voltages);
                for i in 0..num_voltages {
                    let offset = 2 + i * 4;
                    let v = f32::from_le_bytes([
                        data[offset],
                        data[offset + 1],
                        data[offset + 2],
                        data[offset + 3],
                    ]);
                    voltages.push(v);
                }
                Some(Frame::CalibrationData {
                    freq_index,
                    voltages,
                })
            }
            CMD_POWER_DATA if data.len() == 4 => {
                let adc_value =
                    u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
                Some(Frame::PowerData { adc_value })
            }
            _ => None,
        }
    }
}
