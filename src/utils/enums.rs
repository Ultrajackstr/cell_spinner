#[derive(Debug, Copy, Clone)]
pub enum StepMode128 {
    Full,
    M2,
    M4,
    M8,
    M16,
    M32,
    M64,
    M128,
}

impl Default for StepMode128 {
    fn default() -> Self {
        StepMode128::Full
    }
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

#[derive(Debug, Copy, Clone)]
pub enum Direction {
    Forward,
    Backward,
}

impl Default for Direction {
    fn default() -> Self {
        Direction::Forward
    }
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
    EndOfOscillation, //todo: finish
}