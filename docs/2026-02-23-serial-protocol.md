# RPM Power Meter Serial Protocol

Reverse-engineered protocol documentation for RPM-series RF power meters.

## Physical Layer

- **Interface:** USB-to-Serial (appears as `/dev/ttyUSB*` or `/dev/tty.usbserial-*`)
- **Baud rate:** 460,800
- **Data bits:** 8
- **Parity:** None
- **Stop bits:** 1
- **Flow control:** None

## Frame Format

All communication uses a fixed binary frame structure:

```
Byte   Field        Description
─────  ───────────  ──────────────────────────────────
0      SYNC1        0x55 (85)
1      SYNC2        0xAA (170)
2      CMD          Command code
3      LEN          Data payload length (bytes)
4      ~LEN         Bitwise inverse of LEN (validation)
5..N   DATA         Payload (LEN bytes, may be 0)
N+1    CHECKSUM     Sum of bytes 2..N (CMD + LEN + ~LEN + all DATA), truncated to 8 bits
```

Frame validation: verify `SYNC1 == 0x55`, `SYNC2 == 0xAA`, `DATA[4] == ~DATA[3]`, and checksum matches.

## Commands

### QUERY_ID (Host → Device)

Requests device identification.

```
TX:  55 AA 80 00 FF 7F
     ││ ││ ││ ││ ││ └─ checksum: 0x80+0x00+0xFF = 0x17F → 0x7F
     ││ ││ ││ ││ └──── ~LEN = 0xFF
     ││ ││ ││ └─────── LEN = 0 (no payload)
     ││ ││ └────────── CMD = 0x80 (128)
     ││ └───────────── SYNC2
     └──────────────── SYNC1
```

**Response** (CMD = 0x00, LEN = 0x08):

```
RX:  55 AA 00 08 F7 [ID_H] [ID_L] [VER_H] [VER_L] [SN_3] [SN_2] [SN_1] [SN_0] [CHK]
```

| Field | Bytes | Description |
|-------|-------|-------------|
| ID | 2 | Device model ID (101–108), big-endian |
| VER | 2 | Firmware version × 100 (e.g., 123 = v1.23), big-endian |
| SN | 4 | Serial number, big-endian 32-bit unsigned |

### SET_CONFIG (Host → Device)

Sets the sampling rate.

```
TX:  55 AA 83 01 FE [RATE_IDX] [CHK]
```

| RATE_IDX | Sampling Rate |
|----------|---------------|
| 0x00 | 10 Hz |
| 0x01 | 40 Hz |
| 0x02 | 640 Hz |
| 0x03 | 1280 Hz |

**Response** (CMD = 0x03, LEN = 0x01):

```
RX:  55 AA 03 01 FE [RATE_IDX] [CHK]
```

Echoes back the configured rate index as confirmation.

### READ_CALIBRATION (Host → Device)

Reads calibration data for a specific frequency index. Required during initialization — must be called once per frequency point to populate the voltage lookup table.

```
TX:  55 AA 86 02 FD [FREQ_H] [FREQ_L] [CHK]
```

**Response** (CMD = 0x05, LEN = 0x54):

```
RX:  55 AA 05 54 AB [FREQ_H] [FREQ_L] [V0_b0..b3] [V1_b0..b3] ... [V20_b0..b3] [CHK]
```

Returns 21 (or 25, model-dependent) IEEE 754 single-precision float voltage values, one per calibration power level.

### WRITE_CALIBRATION (Host → Device)

Writes calibration data for a frequency index. Not used by the TUI.

```
TX:  55 AA 85 [LEN] [~LEN] [FREQ_H] [FREQ_L] [V0_b0..b3] ... [CHK]
```

LEN = 2 + (num_power_levels × 4).

### POWER_DATA (Device → Host)

Continuous measurement data. Sent by device at the configured sampling rate after initialization completes.

```
RX:  55 AA 06 04 FB [ADC_3] [ADC_2] [ADC_1] [ADC_0] [CHK]
```

| Field | Bytes | Description |
|-------|-------|-------------|
| ADC | 4 | Raw 32-bit ADC reading, big-endian |

**Conversion to voltage:**

```
voltage_mV = (ADC_value × 1500.0) / 8388607.0
```

The 8388607 constant is 2^23 - 1 (23-bit ADC range). Voltage range is 0–1500 mV.

The voltage is then converted to dBm via the model-specific 2D lookup table (frequency × voltage interpolation).

## Initialization Sequence

```
Host                              Device
 │                                   │
 │── QUERY_ID (0x80) ──────────────▶│
 │◀──────────────── ID response (0x00)│  → model, version, serial number
 │                                   │
 │── SET_CONFIG (0x83, rate_idx) ──▶│
 │◀────────────── Config ack (0x03) │  → confirms sampling rate
 │                                   │
 │  (optionally load calibration     │
 │   via READ_CALIBRATION × N,       │
 │   not needed if using hardcoded   │
 │   lookup tables)                  │
 │                                   │
 │◀──── POWER_DATA (0x06) ─────────│  ┐
 │◀──── POWER_DATA (0x06) ─────────│  │ continuous stream
 │◀──── POWER_DATA (0x06) ─────────│  │ at configured rate
 │              ...                  │  ┘
```

## State Machine

The .NET app uses these states during initialization:

| State | Description | Transition |
|-------|-------------|------------|
| 0 | Idle, no device | → 1 on serial port open |
| 1 | Port open, send QUERY_ID | → 2 on ID response |
| 2 | ID received, send SET_CONFIG | → 3 on config ack |
| 3 | Loading calibration (iterative) | → 10 when complete |
| 10 | Ready, streaming POWER_DATA | → 0 on disconnect |

The TUI must also perform state 3 — voltage calibration tables are stored on the device and must be loaded before measurements can be converted to dBm. For a model with N frequency points, this means N sequential READ_CALIBRATION request/response pairs.

## Byte Order

All multi-byte integer fields are **big-endian**. Calibration voltage floats are **IEEE 754 single-precision, little-endian** (4 bytes per float, least-significant byte first).

## Checksum Calculation

```rust
fn checksum(cmd: u8, len: u8, data: &[u8]) -> u8 {
    let inv_len = !len;
    let mut sum = cmd.wrapping_add(len).wrapping_add(inv_len);
    for &b in data {
        sum = sum.wrapping_add(b);
    }
    sum
}
```
