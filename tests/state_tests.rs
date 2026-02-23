// tests/state_tests.rs
use wattson::state::*;

#[test]
fn test_ring_buffer_push_and_iter() {
    let mut buf = RingBuffer::new(4);
    buf.push(1.0);
    buf.push(2.0);
    buf.push(3.0);
    let v: Vec<f64> = buf.iter().copied().collect();
    assert_eq!(v, vec![1.0, 2.0, 3.0]);
}

#[test]
fn test_ring_buffer_wraps() {
    let mut buf = RingBuffer::new(3);
    buf.push(1.0);
    buf.push(2.0);
    buf.push(3.0);
    buf.push(4.0); // overwrites 1.0
    let v: Vec<f64> = buf.iter().copied().collect();
    assert_eq!(v, vec![2.0, 3.0, 4.0]);
}

#[test]
fn test_ring_buffer_empty() {
    let buf = RingBuffer::new(4);
    assert_eq!(buf.len(), 0);
    let v: Vec<f64> = buf.iter().copied().collect();
    assert!(v.is_empty());
}

#[test]
fn test_stats_tracking() {
    let mut stats = Stats::new();
    stats.update(-10.0);
    stats.update(-20.0);
    stats.update(-15.0);
    assert!((stats.max - (-10.0)).abs() < 0.001);
    assert!((stats.min - (-20.0)).abs() < 0.001);
    assert!((stats.avg - (-15.0)).abs() < 0.001);
}

#[test]
fn test_stats_single_value() {
    let mut stats = Stats::new();
    stats.update(-12.5);
    assert!((stats.max - (-12.5)).abs() < 0.001);
    assert!((stats.min - (-12.5)).abs() < 0.001);
    assert!((stats.avg - (-12.5)).abs() < 0.001);
    assert_eq!(stats.count, 1);
}

#[test]
fn test_stats_reset() {
    let mut stats = Stats::new();
    stats.update(-10.0);
    stats.update(-20.0);
    stats.reset();
    assert_eq!(stats.count, 0);
}

#[test]
fn test_format_linear_power_uw() {
    let (value, unit) = format_linear_power(-10.0);
    // -10 dBm = 0.1 mW = 100 uW
    assert!((value - 100.0).abs() < 0.1);
    assert_eq!(unit, "uW");
}

#[test]
fn test_format_linear_power_mw() {
    let (value, unit) = format_linear_power(10.0);
    // 10 dBm = 10 mW
    assert!((value - 10.0).abs() < 0.1);
    assert_eq!(unit, "mW");
}

#[test]
fn test_format_linear_power_nw() {
    let (value, unit) = format_linear_power(-50.0);
    // -50 dBm = 10 nW
    assert!((value - 10.0).abs() < 0.5);
    assert_eq!(unit, "nW");
}

#[test]
fn test_format_linear_power_w() {
    let (value, unit) = format_linear_power(30.0);
    // 30 dBm = 1 W
    assert!((value - 1.0).abs() < 0.01);
    assert_eq!(unit, "W");
}

#[test]
fn test_app_state_push_reading() {
    let mut state = AppState::new();
    state.push_reading(-12.0);
    assert!((state.current_dbm.unwrap() - (-12.0)).abs() < 0.001);
    assert_eq!(state.stats.count, 1);
    assert_eq!(state.waveform.len(), 1);
}

#[test]
fn test_app_state_push_reading_with_offset() {
    let mut state = AppState::new();
    state.offset_db = 2.0;
    state.push_reading(-12.0);
    // Should store adjusted value: -12.0 + 2.0 = -10.0
    assert!((state.current_dbm.unwrap() - (-10.0)).abs() < 0.001);
}

#[test]
fn test_app_state_paused_doesnt_update_stats() {
    let mut state = AppState::new();
    state.push_reading(-12.0);
    state.paused = true;
    state.push_reading(-20.0);
    // Stats should still reflect only the first reading
    assert_eq!(state.stats.count, 1);
    assert_eq!(state.waveform.len(), 1);
    // But current_dbm should still update
    assert!((state.current_dbm.unwrap() - (-20.0)).abs() < 0.001);
}
