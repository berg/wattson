use std::io;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use wattson::device::{DeviceCommand, DeviceEvent};
use wattson::input::{self, Action};
use wattson::logging::CsvLogger;
use wattson::protocol::SamplingRate;
use wattson::state::AppState;
use wattson::ui;

#[derive(Parser)]
#[command(name = "wattson", about = "RF Power Meter TUI")]
struct Cli {
    /// Serial port (e.g., /dev/ttyUSB0 or /dev/tty.usbserial-XXX); auto-detected if omitted
    #[arg(short, long)]
    port: Option<String>,

    /// Initial frequency in MHz
    #[arg(short, long, default_value = "10.0")]
    freq: f64,

    /// Initial sampling rate index (0=10Hz, 1=40Hz, 2=640Hz, 3=1280Hz)
    #[arg(short, long, default_value = "2")]
    rate: u8,

    /// Power offset in dB
    #[arg(short, long, default_value = "0.0")]
    offset: f64,
}

/// Which kind of text input is active
#[derive(Clone, Copy, PartialEq)]
enum InputKind {
    Frequency,
    Offset,
}

/// Input mode for text entry (frequency, offset)
enum InputMode {
    Normal,
    Editing { kind: InputKind, buf: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let initial_rate = SamplingRate::from_index(cli.rate).unwrap_or(SamplingRate::Hz640);

    // Resolve serial port: explicit arg or auto-detect by USB VID/PID
    let port_name = match cli.port {
        Some(p) => {
            // Validate the explicitly specified port exists
            match serialport::available_ports() {
                Ok(ports) => {
                    if !ports.iter().any(|info| info.port_name == p) {
                        eprintln!("Error: serial port '{}' not found.", p);
                        eprintln!();
                        eprintln!("Available ports:");
                        for info in &ports {
                            eprintln!("  {}", info.port_name);
                        }
                        if ports.is_empty() {
                            eprintln!("  (none found)");
                        }
                        std::process::exit(1);
                    }
                }
                Err(_) => {} // Can't enumerate, proceed anyway
            }
            p
        }
        None => {
            match wattson::device::find_device_port() {
                Some(p) => {
                    eprintln!("Auto-detected device on {}", p);
                    p
                }
                None => {
                    eprintln!("Error: no RF power meter found (USB VID=1a86, PID=7523).");
                    eprintln!("Connect the device or specify a port with --port.");
                    std::process::exit(1);
                }
            }
        }
    };

    // Set up channels
    let (event_tx, event_rx) = crossbeam_channel::unbounded::<DeviceEvent>();
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<DeviceCommand>();

    // Spawn serial thread
    let freq_mhz = cli.freq;
    let serial_handle = thread::spawn(move || {
        if let Err(e) = wattson::device::run_serial_thread(
            &port_name,
            initial_rate,
            freq_mhz,
            event_tx,
            cmd_rx,
        ) {
            eprintln!("Serial thread error: {}", e);
        }
    });

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // App state
    let mut state = AppState::new();
    state.frequency_mhz = cli.freq;
    state.offset_db = cli.offset;
    state.sampling_rate = initial_rate;

    let mut input_mode = InputMode::Normal;
    let mut logger: Option<CsvLogger> = None;

    // Main event loop
    let tick_rate = Duration::from_millis(33); // ~30 FPS
    loop {
        // Drain device events
        while let Ok(dev_event) = event_rx.try_recv() {
            match dev_event {
                DeviceEvent::Connected {
                    model_id: _,
                    model_name,
                    firmware_version,
                    serial_number,
                } => {
                    state.model_name = Some(model_name);
                    state.firmware_version = Some(firmware_version);
                    state.serial_number = Some(serial_number);
                    state.connected = true;
                }
                DeviceEvent::CalibrationProgress { loaded, total } => {
                    state.calibration_progress = Some((loaded, total));
                }
                DeviceEvent::Ready => {
                    state.calibration_progress = None;
                }
                DeviceEvent::PowerReading(dbm) => {
                    state.push_reading(dbm);
                    if state.logging {
                        if let Some(ref mut log) = logger {
                            let _ = log.write(
                                state.frequency_mhz,
                                state.current_dbm.unwrap_or(dbm),
                            );
                        }
                    }
                }
                DeviceEvent::Error(msg) => {
                    // Could show in UI status bar in future
                    let _ = msg;
                }
                DeviceEvent::Disconnected => {
                    state.connected = false;
                    state.model_name = None;
                    state.calibration_progress = None;
                }
            }
        }

        // Render
        terminal.draw(|f| ui::draw(f, &state))?;

        // Poll for key events
        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                match input_mode {
                    InputMode::Normal => {
                        match input::handle_key(key) {
                            Action::Quit => break,
                            Action::TogglePause => state.paused = !state.paused,
                            Action::ToggleLogging => {
                                if state.logging {
                                    logger = None;
                                    state.logging = false;
                                } else {
                                    match CsvLogger::new() {
                                        Ok(l) => {
                                            logger = Some(l);
                                            state.logging = true;
                                        }
                                        Err(_) => {} // silently fail
                                    }
                                }
                            }
                            Action::ResetStats => state.stats.reset(),
                            Action::CycleSamplingRate => {
                                let new_rate = state.sampling_rate.next();
                                state.sampling_rate = new_rate;
                                let _ = cmd_tx.send(DeviceCommand::SetSamplingRate(new_rate));
                            }
                            Action::ToggleAutoScale => state.auto_scale = !state.auto_scale,
                            Action::ZoomIn => {
                                state.auto_scale = false;
                                let range = state.y_max - state.y_min;
                                let center = state.current_dbm
                                    .unwrap_or((state.y_max + state.y_min) / 2.0);
                                let new_range = (range * 0.8).max(10.0);
                                let mut lo = center - new_range / 2.0;
                                let mut hi = center + new_range / 2.0;
                                let y_cap = 2.5;
                                if hi > y_cap && state.current_dbm.map_or(true, |v| v <= y_cap) {
                                    hi = y_cap;
                                    lo = hi - new_range;
                                }
                                if lo < -40.0 { lo = -40.0; }
                                state.y_min = lo;
                                state.y_max = hi;
                            }
                            Action::ZoomOut => {
                                state.auto_scale = false;
                                let range = state.y_max - state.y_min;
                                let center = state.current_dbm
                                    .unwrap_or((state.y_max + state.y_min) / 2.0);
                                let new_range = range * 1.25;
                                let mut lo = center - new_range / 2.0;
                                let mut hi = center + new_range / 2.0;
                                let y_cap = 2.5;
                                if hi > y_cap && state.current_dbm.map_or(true, |v| v <= y_cap) {
                                    hi = y_cap;
                                    lo = hi - new_range;
                                }
                                if lo < -40.0 { lo = -40.0; }
                                state.y_min = lo;
                                state.y_max = hi;
                            }
                            Action::ToggleHelp => state.show_help = !state.show_help,
                            Action::StartFrequencyInput => {
                                input_mode = InputMode::Editing {
                                    kind: InputKind::Frequency,
                                    buf: String::new(),
                                };
                                state.input_prompt =
                                    Some(("Freq (MHz): ".into(), String::new()));
                            }
                            Action::StartOffsetInput => {
                                input_mode = InputMode::Editing {
                                    kind: InputKind::Offset,
                                    buf: String::new(),
                                };
                                state.input_prompt =
                                    Some(("Offset (dB): ".into(), String::new()));
                            }
                            Action::None => {}
                        }
                    }
                    InputMode::Editing {
                        ref kind,
                        ref mut buf,
                    } => match key.code {
                        KeyCode::Char(c)
                            if c.is_ascii_digit() || c == '.' || c == '-' =>
                        {
                            buf.push(c);
                            if let Some((_, ref mut prompt_buf)) =
                                state.input_prompt
                            {
                                prompt_buf.push(c);
                            }
                        }
                        KeyCode::Backspace => {
                            buf.pop();
                            if let Some((_, ref mut prompt_buf)) =
                                state.input_prompt
                            {
                                prompt_buf.pop();
                            }
                        }
                        KeyCode::Enter => {
                            if let Ok(val) = buf.parse::<f64>() {
                                match kind {
                                    InputKind::Frequency => {
                                        state.frequency_mhz = val;
                                        let _ =
                                            cmd_tx.send(DeviceCommand::SetFrequency(val));
                                    }
                                    InputKind::Offset => {
                                        state.offset_db = val;
                                    }
                                }
                            }
                            state.input_prompt = None;
                            input_mode = InputMode::Normal;
                        }
                        KeyCode::Esc => {
                            state.input_prompt = None;
                            input_mode = InputMode::Normal;
                        }
                        _ => {}
                    },
                }
            }
        }
    }

    // Cleanup
    let _ = cmd_tx.send(DeviceCommand::Disconnect);
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Wait for serial thread
    let _ = serial_handle.join();

    Ok(())
}
