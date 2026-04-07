use std::io::Read;
use std::time::{Duration, Instant};

const DEVICE_VID: u16 = 0x1a86;
const DEVICE_PID: u16 = 0x7523;

/// Find the serial port for the RF power meter by USB VID/PID.
/// Returns the port name if found, or None.
pub fn find_device_port() -> Option<String> {
    let ports = serialport::available_ports().ok()?;
    ports.into_iter().find_map(|p| {
        if let serialport::SerialPortType::UsbPort(info) = p.port_type {
            if info.vid == DEVICE_VID && info.pid == DEVICE_PID {
                return Some(p.port_name);
            }
        }
        None
    })
}

use crate::model::{adc_to_voltage, DeviceCalibration, ModelSpec};
use crate::protocol::{
    build_query_id, build_read_calibration, build_set_config, Frame, FrameParser, SamplingRate,
};

/// Messages sent from the serial thread to the main thread
#[derive(Debug)]
pub enum DeviceEvent {
    Connected {
        model_id: u16,
        model_name: String,
        firmware_version: String,
        serial_number: u32,
    },
    CalibrationProgress {
        loaded: usize,
        total: usize,
    },
    Ready,
    PowerReading(f64),
    Error(String),
    Disconnected,
}

/// Messages sent from main thread to serial thread
#[derive(Debug)]
pub enum DeviceCommand {
    SetSamplingRate(SamplingRate),
    SetFrequency(f64), // MHz
    Disconnect,
}

/// Initialization state machine states
enum InitState {
    QueryId,
    ConfigSet {
        model_id: u16,
    },
    LoadCalibration {
        calibration: DeviceCalibration,
        freq_index: usize,
        freq_count: usize,
    },
}

/// Read available bytes from the serial port, feeding them into the parser.
/// Returns `Err` only on fatal read errors (not timeouts).
fn read_serial(
    port: &mut Box<dyn serialport::SerialPort>,
    parser: &mut FrameParser,
    read_buf: &mut [u8],
) -> Result<(), String> {
    match port.read(read_buf) {
        Ok(n) if n > 0 => {
            parser.feed(&read_buf[..n]);
        }
        Ok(_) => {} // no data
        Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {} // normal timeout
        Err(e) => {
            return Err(format!("Serial read error: {}", e));
        }
    }
    Ok(())
}

/// Wait for a specific frame type with a timeout, reading from the port and
/// feeding the parser. Returns the first matching frame, or None on timeout.
fn wait_for_frame<F>(
    port: &mut Box<dyn serialport::SerialPort>,
    parser: &mut FrameParser,
    read_buf: &mut [u8],
    timeout: Duration,
    matcher: F,
) -> Result<Option<Frame>, String>
where
    F: Fn(&Frame) -> bool,
{
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        read_serial(port, parser, read_buf)?;
        while let Some(frame) = parser.next_frame() {
            if matcher(&frame) {
                return Ok(Some(frame));
            }
        }
    }
    Ok(None)
}

/// Format a firmware version u16 into a human-readable string.
/// The firmware version is encoded as major * 100 + minor.
fn format_firmware_version(fw: u16) -> String {
    format!("{}.{:02}", fw / 100, fw % 100)
}

/// Runs in the serial thread. Handles:
/// 1. Open serial port at 460800 baud, 8N1
/// 2. Send QUERY_ID, parse response
/// 3. Send SET_CONFIG with default rate
/// 4. Load calibration (READ_CALIBRATION x freq_count)
/// 5. Enter streaming mode -- convert ADC to dBm, send PowerReading events
pub fn run_serial_thread(
    port_name: &str,
    initial_rate: SamplingRate,
    initial_freq_mhz: f64,
    event_tx: crossbeam_channel::Sender<DeviceEvent>,
    cmd_rx: crossbeam_channel::Receiver<DeviceCommand>,
) -> anyhow::Result<()> {
    use std::io::Write;

    // 1. Open serial port
    let mut port = serialport::new(port_name, 460_800)
        .timeout(Duration::from_millis(100))
        .open()?;

    let mut parser = FrameParser::new();
    let mut read_buf = [0u8; 256];

    // 2. Initialization: QueryID
    let mut state = InitState::QueryId;

    loop {
        match state {
            InitState::QueryId => {
                // Send QUERY_ID and wait for DeviceId response
                let query_frame = build_query_id();
                port.write_all(&query_frame)?;

                match wait_for_frame(
                    &mut port,
                    &mut parser,
                    &mut read_buf,
                    Duration::from_secs(3),
                    |f| matches!(f, Frame::DeviceId { .. }),
                ) {
                    Ok(Some(Frame::DeviceId {
                        model_id,
                        firmware_version,
                        serial_number,
                    })) => {
                        let model_name = ModelSpec::from_id(model_id)
                            .map(|s| s.name.to_string())
                            .unwrap_or_else(|| format!("Unknown ({})", model_id));

                        event_tx
                            .send(DeviceEvent::Connected {
                                model_id,
                                model_name,
                                firmware_version: format_firmware_version(firmware_version),
                                serial_number,
                            })
                            .ok();

                        state = InitState::ConfigSet { model_id };
                    }
                    Ok(_) => {
                        // Timeout -- report error and retry
                        event_tx
                            .send(DeviceEvent::Error(
                                "No response to device ID query, retrying...".into(),
                            ))
                            .ok();
                        // Loop back to QueryId (state is already QueryId)
                        continue;
                    }
                    Err(e) => {
                        event_tx.send(DeviceEvent::Error(e.clone())).ok();
                        event_tx.send(DeviceEvent::Disconnected).ok();
                        return Ok(());
                    }
                }
            }

            InitState::ConfigSet { model_id } => {
                // Create calibration container
                let calibration = match DeviceCalibration::new(model_id) {
                    Some(cal) => cal,
                    None => {
                        event_tx
                            .send(DeviceEvent::Error(format!(
                                "Unknown model ID: {}",
                                model_id
                            )))
                            .ok();
                        event_tx.send(DeviceEvent::Disconnected).ok();
                        return Ok(());
                    }
                };

                // Send SET_CONFIG
                let config_frame = build_set_config(initial_rate);
                port.write_all(&config_frame)?;

                match wait_for_frame(
                    &mut port,
                    &mut parser,
                    &mut read_buf,
                    Duration::from_secs(3),
                    |f| matches!(f, Frame::ConfigAck { .. }),
                ) {
                    Ok(Some(Frame::ConfigAck { .. })) => {
                        let freq_count = calibration.spec.freq_count;
                        state = InitState::LoadCalibration {
                            calibration,
                            freq_index: 0,
                            freq_count,
                        };
                    }
                    Ok(_) => {
                        event_tx
                            .send(DeviceEvent::Error(
                                "No config acknowledgment, retrying...".into(),
                            ))
                            .ok();
                        // Retry by staying in ConfigSet
                        continue;
                    }
                    Err(e) => {
                        event_tx.send(DeviceEvent::Error(e.clone())).ok();
                        event_tx.send(DeviceEvent::Disconnected).ok();
                        return Ok(());
                    }
                }
            }

            InitState::LoadCalibration {
                ref mut calibration,
                ref mut freq_index,
                freq_count,
            } => {
                // Load calibration data row by row
                while *freq_index < freq_count {
                    let idx = *freq_index as u16;
                    let cal_frame = build_read_calibration(idx);
                    port.write_all(&cal_frame)?;

                    let mut retries = 0;
                    let max_retries = 3;
                    let mut got_row = false;

                    while retries < max_retries {
                        match wait_for_frame(
                            &mut port,
                            &mut parser,
                            &mut read_buf,
                            Duration::from_secs(3),
                            |f| matches!(f, Frame::CalibrationData { .. }),
                        ) {
                            Ok(Some(Frame::CalibrationData {
                                freq_index: resp_idx,
                                voltages,
                            })) => {
                                if resp_idx == idx {
                                    calibration
                                        .set_calibration_row(*freq_index, voltages);
                                    got_row = true;
                                    break;
                                }
                                // Got a different index, retry
                                retries += 1;
                            }
                            Ok(_) => {
                                // Timeout, retry
                                retries += 1;
                                port.write_all(&cal_frame)?;
                            }
                            Err(e) => {
                                event_tx.send(DeviceEvent::Error(e.clone())).ok();
                                event_tx.send(DeviceEvent::Disconnected).ok();
                                return Ok(());
                            }
                        }
                    }

                    if !got_row {
                        event_tx
                            .send(DeviceEvent::Error(format!(
                                "Failed to load calibration row {} after {} retries",
                                freq_index, max_retries
                            )))
                            .ok();
                        event_tx.send(DeviceEvent::Disconnected).ok();
                        return Ok(());
                    }

                    *freq_index += 1;
                    event_tx
                        .send(DeviceEvent::CalibrationProgress {
                            loaded: *freq_index,
                            total: freq_count,
                        })
                        .ok();
                }

                // All calibration rows loaded -- transition to streaming
                event_tx.send(DeviceEvent::Ready).ok();

                // Move calibration out of the borrow by rebuilding state
                // We need to break out of the match to transition
                break;
            }

        }
    }

    // Extract calibration from the LoadCalibration state
    // At this point, `state` is LoadCalibration with all data loaded.
    let calibration = match state {
        InitState::LoadCalibration { calibration, .. } => calibration,
        _ => unreachable!("Expected LoadCalibration state after init loop"),
    };

    let mut current_freq_mhz = initial_freq_mhz;

    // 5. Streaming loop
    loop {
        // Read from serial port
        match read_serial(&mut port, &mut parser, &mut read_buf) {
            Ok(()) => {}
            Err(e) => {
                event_tx.send(DeviceEvent::Error(e)).ok();
                event_tx.send(DeviceEvent::Disconnected).ok();
                return Ok(());
            }
        }

        // Process frames
        while let Some(frame) = parser.next_frame() {
            if let Frame::PowerData { adc_value } = frame {
                let voltage = adc_to_voltage(adc_value);
                let freq_hz = (current_freq_mhz * 1_000_000.0) as i64;
                match calibration.convert(freq_hz, voltage) {
                    Some(dbm) => {
                        event_tx.send(DeviceEvent::PowerReading(dbm as f64)).ok();
                    }
                    None => {
                        // Over-range or out-of-frequency-range
                        event_tx.send(DeviceEvent::PowerReading(f64::NAN)).ok();
                    }
                }
            }
        }

        // Check for commands
        if let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                DeviceCommand::SetSamplingRate(rate) => {
                    let config_frame = build_set_config(rate);
                    if let Err(e) = port.write_all(&config_frame) {
                        event_tx
                            .send(DeviceEvent::Error(format!(
                                "Failed to set sampling rate: {}",
                                e
                            )))
                            .ok();
                    }
                }
                DeviceCommand::SetFrequency(freq_mhz) => {
                    current_freq_mhz = freq_mhz;
                }
                DeviceCommand::Disconnect => {
                    event_tx.send(DeviceEvent::Disconnected).ok();
                    return Ok(());
                }
            }
        }
    }
}
