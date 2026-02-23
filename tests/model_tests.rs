use wattson::model::*;

#[test]
fn test_model_from_id() {
    let spec = ModelSpec::from_id(101).unwrap();
    assert_eq!(spec.name, "RPM-20GS");
    assert_eq!(spec.freq_count, 202);
    assert_eq!(spec.power_count, 21);
    assert!((spec.power_min - (-40.0)).abs() < 0.01);

    let spec107 = ModelSpec::from_id(107).unwrap();
    assert_eq!(spec107.name, "RPM-6GH");
    assert_eq!(spec107.freq_count, 69);
    assert_eq!(spec107.power_count, 41);
    assert!((spec107.power_min - (-80.0)).abs() < 0.01);

    assert!(ModelSpec::from_id(999).is_none());
}

#[test]
fn test_freq_table_101() {
    let spec = ModelSpec::from_id(101).unwrap();
    let freqs = spec.build_freq_table();
    assert_eq!(freqs.len(), 202);
    assert_eq!(freqs[0], 10_000_000);
    assert_eq!(freqs[1], 50_000_000);
    assert_eq!(freqs[2], 100_000_000);
    assert_eq!(freqs[3], 200_000_000);
    assert_eq!(freqs[201], 20_000_000_000);
}

#[test]
fn test_freq_table_102() {
    let spec = ModelSpec::from_id(102).unwrap();
    let freqs = spec.build_freq_table();
    assert_eq!(freqs.len(), 33);
    assert_eq!(freqs[0], 50);
    assert_eq!(freqs[1], 10_000_000);
    assert_eq!(freqs[2], 50_000_000);
    assert_eq!(freqs[3], 100_000_000);
}

#[test]
fn test_freq_table_104() {
    let spec = ModelSpec::from_id(104).unwrap();
    let freqs = spec.build_freq_table();
    assert_eq!(freqs.len(), 218);
    assert_eq!(freqs[0], 10_000_000);
    assert_eq!(freqs[1], 15_000_000);
    assert_eq!(freqs[17], 95_000_000);
    assert_eq!(freqs[18], 100_000_000);
}

#[test]
fn test_freq_table_105() {
    let spec = ModelSpec::from_id(105).unwrap();
    let freqs = spec.build_freq_table();
    assert_eq!(freqs.len(), 58);
    assert_eq!(freqs[0], 50);           // 50 Hz
    assert_eq!(freqs[1], 100_000);      // 100 kHz
    assert_eq!(freqs[9], 900_000);      // 900 kHz
    assert_eq!(freqs[10], 1_000_000);   // 1 MHz
    assert_eq!(freqs[18], 9_000_000);   // 9 MHz
    assert_eq!(freqs[19], 10_000_000);  // 10 MHz
    assert_eq!(freqs[27], 90_000_000);  // 90 MHz
    assert_eq!(freqs[28], 100_000_000); // 100 MHz
    assert_eq!(freqs[57], 3_000_000_000); // 3000 MHz
}

#[test]
fn test_freq_table_106() {
    let spec = ModelSpec::from_id(106).unwrap();
    let freqs = spec.build_freq_table();
    assert_eq!(freqs.len(), 99);
    assert_eq!(freqs[0], 10_000_000);
    assert_eq!(freqs[8], 90_000_000);
    assert_eq!(freqs[9], 100_000_000);
}

#[test]
fn test_freq_table_107() {
    let spec = ModelSpec::from_id(107).unwrap();
    let freqs = spec.build_freq_table();
    assert_eq!(freqs.len(), 69);
    assert_eq!(freqs[0], 10_000_000);
    assert_eq!(freqs[8], 90_000_000);
    assert_eq!(freqs[9], 100_000_000);
    assert_eq!(freqs[68], 6_000_000_000);
}

#[test]
fn test_freq_table_108() {
    let spec = ModelSpec::from_id(108).unwrap();
    let freqs = spec.build_freq_table();
    assert_eq!(freqs.len(), 283);
    assert_eq!(freqs[0], 10_000_000);
    assert_eq!(freqs[17], 95_000_000);
    assert_eq!(freqs[18], 100_000_000);
    assert_eq!(freqs[282], 26_500_000_000);
}

#[test]
fn test_power_table() {
    let spec = ModelSpec::from_id(101).unwrap();
    let powers = spec.build_power_table();
    assert_eq!(powers.len(), 21);
    assert!((powers[0] - (-40.0)).abs() < 0.01);
    assert!((powers[1] - (-37.5)).abs() < 0.01);
    assert!((powers[20] - 10.0).abs() < 0.01);
}

#[test]
fn test_power_table_107() {
    let spec = ModelSpec::from_id(107).unwrap();
    let powers = spec.build_power_table();
    assert_eq!(powers.len(), 41);
    assert!((powers[0] - (-80.0)).abs() < 0.01);
    assert!((powers[40] - 20.0).abs() < 0.01);
}

#[test]
fn test_adc_to_voltage() {
    assert!((adc_to_voltage(8388607) - 1500.0).abs() < 0.01);
    assert!((adc_to_voltage(0) - 0.0).abs() < 0.01);
    assert!((adc_to_voltage(4194303) - 750.0).abs() < 0.1);
}

#[test]
fn test_interpolate_power_basic() {
    let freq_table = vec![100_000_000i64, 200_000_000];
    let power_table = vec![-40.0f32, -37.5, -35.0];
    let voltage_table = vec![
        vec![10.0f32, 50.0, 100.0],
        vec![12.0f32, 55.0, 110.0],
    ];
    let result = interpolate_power(
        100_000_000,
        30.0,
        &freq_table,
        &power_table,
        &voltage_table,
        -40.0,
    );
    assert!(result.is_some());
    let p = result.unwrap();
    assert!(p > -40.0 && p < -37.5);
}

#[test]
fn test_interpolate_power_out_of_range() {
    let freq_table = vec![100_000_000i64, 200_000_000];
    let power_table = vec![-40.0f32, -37.5, -35.0];
    let voltage_table = vec![
        vec![10.0f32, 50.0, 100.0],
        vec![12.0f32, 55.0, 110.0],
    ];
    let result = interpolate_power(
        50_000_000,
        30.0,
        &freq_table,
        &power_table,
        &voltage_table,
        -40.0,
    );
    assert!(result.is_none());
}

#[test]
fn test_interpolate_between_frequencies() {
    let freq_table = vec![100_000_000i64, 200_000_000];
    let power_table = vec![-40.0f32, -37.5, -35.0];
    let voltage_table = vec![
        vec![10.0f32, 50.0, 100.0],
        vec![10.0f32, 50.0, 100.0], // same voltages at both freqs
    ];
    // Midpoint frequency, midpoint voltage
    let result = interpolate_power(
        150_000_000,
        30.0,
        &freq_table,
        &power_table,
        &voltage_table,
        -40.0,
    );
    assert!(result.is_some());
    let p = result.unwrap();
    // Should be same regardless of frequency since voltage tables are identical
    assert!(p > -40.0 && p < -37.5);
}

#[test]
fn test_device_calibration() {
    let cal = DeviceCalibration::new(101).unwrap();
    assert_eq!(cal.freq_table.len(), 202);
    assert_eq!(cal.power_table.len(), 21);
    assert_eq!(cal.voltage_table.len(), 202);
    assert_eq!(cal.voltage_table[0].len(), 21);
}
