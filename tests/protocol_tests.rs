use wattson::protocol::*;

// ─── Frame Builder Tests ────────────────────────────────────────────

#[test]
fn test_checksum_query_id() {
    // CMD=0x80, LEN=0, no data
    // checksum = 0x80 + 0x00 + 0xFF = 0x17F -> 0x7F
    assert_eq!(checksum(0x80, 0x00, &[]), 0x7F);
}

#[test]
fn test_build_query_id_frame() {
    let frame = build_query_id();
    assert_eq!(frame, vec![0x55, 0xAA, 0x80, 0x00, 0xFF, 0x7F]);
}

#[test]
fn test_build_set_config_frame() {
    let frame = build_set_config(SamplingRate::Hz640);
    assert_eq!(frame[0..2], [0x55, 0xAA]);
    assert_eq!(frame[2], 0x83); // CMD
    assert_eq!(frame[3], 0x01); // LEN
    assert_eq!(frame[4], 0xFE); // ~LEN
    assert_eq!(frame[5], 0x02); // rate index for 640Hz
}

#[test]
fn test_build_read_calibration_frame() {
    let frame = build_read_calibration(0x0005); // freq index 5
    assert_eq!(frame[0..2], [0x55, 0xAA]);
    assert_eq!(frame[2], 0x86); // CMD
    assert_eq!(frame[3], 0x02); // LEN
    assert_eq!(frame[4], 0xFD); // ~LEN
    assert_eq!(frame[5], 0x00); // freq index high byte
    assert_eq!(frame[6], 0x05); // freq index low byte
}

// ─── Frame Parser Tests ─────────────────────────────────────────────

#[test]
fn test_parse_id_response() {
    let mut data = vec![0x55, 0xAA, 0x00, 0x08, 0xF7];
    let payload: Vec<u8> = vec![
        0x00, 0x65, // ID = 101
        0x00, 0x7B, // VER = 123 -> v1.23
        0x00, 0x00, 0x30, 0x39, // SN = 12345
    ];
    data.extend_from_slice(&payload);
    data.push(checksum(0x00, 0x08, &payload));

    let mut parser = FrameParser::new();
    parser.feed(&data);
    let frame = parser.next_frame().unwrap();
    match frame {
        Frame::DeviceId {
            model_id,
            firmware_version,
            serial_number,
        } => {
            assert_eq!(model_id, 101);
            assert_eq!(firmware_version, 123);
            assert_eq!(serial_number, 12345);
        }
        _ => panic!("expected DeviceId frame"),
    }
}

#[test]
fn test_parse_power_data() {
    let mut data = vec![0x55, 0xAA, 0x06, 0x04, 0xFB];
    let payload: Vec<u8> = vec![0x00, 0x3F, 0xFF, 0xFF];
    data.extend_from_slice(&payload);
    data.push(checksum(0x06, 0x04, &payload));

    let mut parser = FrameParser::new();
    parser.feed(&data);
    let frame = parser.next_frame().unwrap();
    match frame {
        Frame::PowerData { adc_value } => {
            assert_eq!(adc_value, 0x003FFFFF);
        }
        _ => panic!("expected PowerData frame"),
    }
}

#[test]
fn test_parse_config_ack() {
    let mut data = vec![0x55, 0xAA, 0x03, 0x01, 0xFE];
    let payload: Vec<u8> = vec![0x02]; // 640 Hz
    data.extend_from_slice(&payload);
    data.push(checksum(0x03, 0x01, &payload));

    let mut parser = FrameParser::new();
    parser.feed(&data);
    let frame = parser.next_frame().unwrap();
    match frame {
        Frame::ConfigAck { rate } => {
            assert_eq!(rate, SamplingRate::Hz640);
        }
        _ => panic!("expected ConfigAck frame"),
    }
}

#[test]
fn test_parse_calibration_response() {
    let mut payload = vec![0x00, 0x05]; // freq index 5
    for i in 0..21u32 {
        let v = (i as f32) * 10.0;
        payload.extend_from_slice(&v.to_le_bytes());
    }
    let len = payload.len() as u8;
    let mut data = vec![0x55, 0xAA, 0x05, len, !len];
    data.extend_from_slice(&payload);
    data.push(checksum(0x05, len, &payload));

    let mut parser = FrameParser::new();
    parser.feed(&data);
    let frame = parser.next_frame().unwrap();
    match frame {
        Frame::CalibrationData {
            freq_index,
            voltages,
        } => {
            assert_eq!(freq_index, 5);
            assert_eq!(voltages.len(), 21);
            assert!((voltages[0] - 0.0).abs() < 0.001);
            assert!((voltages[2] - 20.0).abs() < 0.001);
        }
        _ => panic!("expected CalibrationData frame"),
    }
}

#[test]
fn test_parser_handles_partial_data() {
    let full = build_query_id();
    let mut parser = FrameParser::new();
    for &b in &full[..full.len() - 1] {
        parser.feed(&[b]);
        assert!(parser.next_frame().is_none());
    }
    parser.feed(&[0x00, 0x00, 0x55, 0xAA]);
    // No crash = pass
}

#[test]
fn test_parser_resyncs_after_garbage() {
    let mut data = vec![0xFF, 0xFF, 0x00, 0x55]; // garbage
    let payload: Vec<u8> = vec![0x00, 0x00, 0x10, 0x00]; // ADC = 4096
    data.extend_from_slice(&[0x55, 0xAA, 0x06, 0x04, 0xFB]);
    data.extend_from_slice(&payload);
    data.push(checksum(0x06, 0x04, &payload));

    let mut parser = FrameParser::new();
    parser.feed(&data);
    let frame = parser.next_frame().unwrap();
    match frame {
        Frame::PowerData { adc_value } => {
            assert_eq!(adc_value, 0x00001000);
        }
        _ => panic!("expected PowerData"),
    }
}
