use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub enum Action {
    Quit,
    TogglePause,
    ToggleLogging,
    ResetStats,
    CycleSamplingRate,
    ToggleAutoScale,
    ZoomIn,
    ZoomOut,
    ToggleHelp,
    StartFrequencyInput,
    StartOffsetInput,
    None,
}

pub fn handle_key(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
        KeyCode::Char(' ') => Action::TogglePause,
        KeyCode::Char('l') => Action::ToggleLogging,
        KeyCode::Char('r') => Action::ResetStats,
        KeyCode::Char('s') => Action::CycleSamplingRate,
        KeyCode::Char('a') => Action::ToggleAutoScale,
        KeyCode::Char('+') | KeyCode::Char('=') => Action::ZoomIn,
        KeyCode::Char('-') => Action::ZoomOut,
        KeyCode::Char('?') => Action::ToggleHelp,
        KeyCode::Char('f') => Action::StartFrequencyInput,
        KeyCode::Char('o') => Action::StartOffsetInput,
        _ => Action::None,
    }
}
