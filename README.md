# wattson

A cross-platform TUI for RPM-series RF power meters. Replaces the Windows-only .NET app that ships with the device.

Connects to the meter over USB serial, loads its calibration data, and streams real-time power readings with a live braille waveform chart, running statistics, and optional CSV logging.

## Hardware

This was built for [this RF power meter from AliExpress](https://www.aliexpress.us/item/3256808269478273.html) (I bought one; no relationship with the seller). The device enumerates as a USB serial port and speaks a binary protocol at 460800 baud.

Supported models:

| Model | Frequency Range | Power Range | Sensor |
|-------|----------------|-------------|--------|
| RPM-3GS | 50 Hz -- 3 GHz | -50 to +10 dBm | AD8362 |
| RPM-9G | 10 MHz -- 9 GHz | -40 to +10 dBm | ARW22347 |
| RPM-20GS | 10 MHz -- 20 GHz | -40 to +10 dBm | ARW28340 |
| RPM-6GH | 10 MHz -- 6 GHz | -80 to +20 dBm | ARW22283 |
| RPM-20GS OPT260 | 10 MHz -- 26.5 GHz | -40 to +10 dBm | ARW28340 |

v2 revisions of the 3GS, 9G, and 20GS are also supported (model IDs 104--106).

## Building

Requires Rust (edition 2021).

```
cargo build --release
```

The binary lands in `target/release/wattson`.

## Usage

```
wattson [--port <SERIAL_PORT>] [--freq <MHz>] [--rate <0-3>] [--offset <dB>] [--mini]
```

**Arguments:**

- `--port` / `-p` -- Serial port path, e.g. `/dev/ttyUSB0` or `/dev/tty.usbserial-1130`. If omitted, wattson auto-detects the device by USB VID/PID (VID `1a86`, PID `7523` — the CH340 chip used by this meter).
- `--freq` / `-f` -- Initial frequency in MHz (default: 10.0)
- `--rate` / `-r` -- Sampling rate index (default: 2)
  - `0` = 10 Hz, `1` = 40 Hz, `2` = 640 Hz, `3` = 1280 Hz
- `--offset` / `-o` -- Power offset in dB (default: 0.0)
- `--mini` / `-m` -- Minimal mode: print readings to a single line in the current terminal without taking over the screen (no TUI, no alternate screen). Useful for quick checks or piping into other tools. Press Ctrl+C to exit.

On startup, wattson queries the device for its model and firmware version, sets the sampling rate, then loads the full calibration table from the device. A progress bar shows during calibration loading. Once complete, the live waveform view starts.

## Key Bindings

| Key | Action |
|-----|--------|
| `q` / `Ctrl+C` | Quit |
| `Space` | Pause / resume |
| `f` | Set frequency (MHz) |
| `o` | Set power offset (dB) |
| `s` | Cycle sampling rate |
| `l` | Toggle CSV logging |
| `r` | Reset min/max/avg statistics |
| `a` | Toggle auto-scale Y-axis |
| `+` / `-` | Zoom in / out (manual Y-axis) |
| `?` | Toggle help overlay |

When entering frequency or offset, type the value and press Enter to confirm or Esc to cancel.

## CSV Logging

Press `l` to start logging. A file named `wattson_YYYYMMDD_HHMMSS.csv` is created in the current directory with columns:

```
timestamp,frequency_mhz,power_dbm,power_w
```

Rows are written at the current sampling rate. Press `l` again to stop.

## License

MIT
