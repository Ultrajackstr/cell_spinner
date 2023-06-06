#[derive(Debug, Copy, Clone, Default)]
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
}

#[derive(Debug, Copy, Clone, Default)]
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

pub enum StepperState {
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