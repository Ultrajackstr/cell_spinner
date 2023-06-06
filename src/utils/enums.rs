use std::fmt::Display;
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum StepMode128 {
    #[default]
    Full,
    M2,
    M4,
    M8,
    M16,
    M32,
    M64,
    M128,
}

impl StepMode128 {
    pub fn to_byte(&self) -> u8 {
        match self {
            StepMode128::Full => 0,
            StepMode128::M2 => 1,
            StepMode128::M4 => 2,
            StepMode128::M8 => 3,
            StepMode128::M16 => 4,
            StepMode128::M32 => 5,
            StepMode128::M64 => 6,
            StepMode128::M128 => 7,
        }
    }

    pub fn get_modes(&self) -> Vec<StepMode128> {
        vec![StepMode128::Full, StepMode128::M2, StepMode128::M4, StepMode128::M8, StepMode128::M16, StepMode128::M32, StepMode128::M64, StepMode128::M128]
    }
}


impl Display for StepMode128 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StepMode128::Full => write!(f, "Full"),
            StepMode128::M2 => write!(f, "1/2"),
            StepMode128::M4 => write!(f, "1/4"),
            StepMode128::M8 => write!(f, "1/8"),
            StepMode128::M16 => write!(f, "1/16"),
            StepMode128::M32 => write!(f, "1/32"),
            StepMode128::M64 => write!(f, "1/64"),
            StepMode128::M128 => write!(f, "1/128"),
        }
    }
}

#[derive(Debug, Copy, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum Direction {
    #[default]
    Forward,
    Backward,
}

impl Direction {
    pub fn to_byte(&self) -> u8 {
        match self {
            Direction::Forward => 0,
            Direction::Backward => 1,
        }
    }
}

impl Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Direction::Forward => write!(f, "Forward"),
            Direction::Backward => write!(f, "Backward"),
        }
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub enum StepperState {
    #[default]
    CommandReceived,
    Finished,
    EmergencyStop,
    OpenLoad,
    OverCurrent,
    OverHeat,
    ParseError,
    OscillationRotation,
    OscillationAgitation,
    Invalid,
}

impl From<&[u8; 3]> for StepperState {
    fn from(bytes: &[u8; 3]) -> Self {
        match bytes {
            [b'o', b'k', b'!'] => StepperState::CommandReceived,
            [b'f', b'i', b'n'] => StepperState::Finished,
            [b'e', b'm', b'r'] => StepperState::EmergencyStop,
            [b'e', b'r', b'1'] => StepperState::OpenLoad,
            [b'e', b'r', b'2'] => StepperState::OverCurrent,
            [b'e', b'r', b'3'] => StepperState::OverHeat,
            [b'e', b'r', b'p'] => StepperState::ParseError,
            [b'o', b's', b'r'] => StepperState::OscillationRotation,
            [b'o', b's', b'a'] => StepperState::OscillationAgitation,
            _ => StepperState::Invalid,
        }
    }
}

impl Display for StepperState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StepperState::CommandReceived => write!(f, "Command received"),
            StepperState::Finished => write!(f, "Finished"),
            StepperState::EmergencyStop => write!(f, "Emergency stop"),
            StepperState::OpenLoad => write!(f, "Open load"),
            StepperState::OverCurrent => write!(f, "Over current"),
            StepperState::OverHeat => write!(f, "Over heat"),
            StepperState::ParseError => write!(f, "Parse error"),
            StepperState::OscillationRotation => write!(f, "Oscillation rotation"),
            StepperState::OscillationAgitation => write!(f, "Oscillation agitation"),
            StepperState::Invalid => write!(f, "Invalid"),
        }
    }
}