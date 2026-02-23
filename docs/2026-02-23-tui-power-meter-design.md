# RF Power Meter TUI — Design Document

## Overview

A Rust TUI application that replaces the Windows-only .NET RF Power Meter app. Talks directly to RPM-series RF power meters over serial, displays real-time power measurements with a full-width braille waveform chart, and logs to CSV.

Target platforms: macOS and Linux.

## Supported Hardware

| Model ID | Name | Freq Range | Power Range | Sensor IC |
|----------|------|------------|-------------|-----------|
| 101 | RPM-20GS | 10 MHz – 20 GHz | -40 to +10 dBm | ARW28340 |
| 102 | RPM-3GS | 50 Hz – 3 GHz | -50 to +10 dBm | AD8362 |
| 103 | RPM-9G | 10 MHz – 9 GHz | -40 to +10 dBm | ARW22347 |
| 104 | RPM-20GS v2 | 10 MHz – 20 GHz | -40 to +10 dBm | ARW28340 |
| 105 | RPM-3GS v2 | 50 Hz – 3 GHz | -50 to +10 dBm | AD8362 |
| 106 | RPM-9G v2 | 10 MHz – 9 GHz | -40 to +10 dBm | ARW22347 |
| 107 | RPM-6GH | 10 MHz – 6 GHz | -80 to +20 dBm | ARW22283 |
| 108 | RPM-20GS OPT260 | 10 MHz – 26.5 GHz | -40 to +10 dBm | ARW28340 |

## Architecture

```
┌─────────────────────────────────────┐
│           TUI Layer (Ratatui)       │
│  Layout, widgets, key handling      │
├─────────────────────────────────────┤
│         Application State           │
│  Ring buffer, stats, config, log    │
├─────────────────────────────────────┤
│        Serial/Protocol Layer        │
│  Frame parser, commands, device     │
└─────────────────────────────────────┘
```

**Two threads:**
1. **Serial thread** — reads serial port, parses binary frames, converts ADC→dBm via lookup tables, sends `f64` values over a `crossbeam` channel.
2. **Main thread** — Ratatui event loop at ~30 FPS. Drains channel each tick, updates state, renders.

**Crates:**
- `ratatui` + `crossterm` — TUI rendering
- `serialport` — serial communication
- `crossbeam-channel` — thread communication
- `csv` — data logging
- `clap` — CLI argument parsing

## UI Layout

```
┌─ RF Power Meter ── RPM-20GS ── S/N: 12345 ── v1.23 ─── 640 Hz ──┐
│                                                                    │
│  Avg: -12.45 dBm  (56.9 uW)   Cur: -12.34 dBm  (58.3 uW)       │
│  Min: -15.67 dBm  (2.71 uW)   Max: -10.21 dBm  (95.5 uW)       │
│                                                                    │
├─ Waveform ────────────────────────────────────────────────────────┤
│     (full-width braille chart, auto-scaling Y axis)               │
│     (scrolling time window matching sampling rate)                │
├────────────────────────────────────────────────────────────────────┤
│ Freq: 2400.0 MHz  Offset: +0.0 dB  Log: OFF     [?] Help        │
└────────────────────────────────────────────────────────────────────┘
```

**Sections:**
- **Title bar** — device model, serial number, firmware version, current sampling rate
- **Readings** — two rows: Avg/Cur on top, Min/Max on bottom. Each shows dBm with linear unit in parens, auto-selecting best prefix (nW/uW/mW/W)
- **Waveform** — full-width Ratatui `Chart` with braille `Marker::Braille`, auto-scaling Y axis, scrolling window of 1280 samples
- **Status bar** — frequency, offset, logging state, help hint

**Key bindings:**
- `q` — quit
- `f` — set frequency (MHz)
- `o` — set power offset (dB)
- `s` — cycle sampling rate (10/40/640/1280 Hz)
- `l` — toggle CSV logging
- `r` — reset min/max/avg stats
- `space` — pause/resume measurement
- `+`/`-` — manual Y-axis zoom
- `a` — toggle auto-scale Y axis
- `?` — help overlay

## ADC to Power Conversion

```
Raw ADC (32-bit int)
    │
    ▼
voltage_mV = (ADC × 1500) / 8388607      [0–1500 mV range]
    │
    ▼
2D interpolation across:
  - frequency table (model-specific, 33–283 points)
  - voltage→power calibration (21–41 power levels)
    │
    ▼
power_dBm = interpolated_value + user_offset
```

Each model has algorithmically-generated frequency and power tables (these become `const` in Rust). The voltage calibration table (the 2D mapping of frequency × voltage → power) is loaded from the device during initialization via READ_CALIBRATION commands — one request per frequency point. This means initialization takes a few seconds for models with many frequency points (e.g., model 108 has 283 freq points).

## CSV Logging

When enabled, writes timestamped rows:

```
timestamp,frequency_mhz,power_dbm,power_linear_w
2026-02-23T14:30:01.123,2400.0,-12.34,5.83e-5
```

File named `wattson_YYYYMMDD_HHMMSS.csv`, created in the current directory.
