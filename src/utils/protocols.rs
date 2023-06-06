use crate::app::BYTES;
use crate::utils::enums::{Direction, StepMode128};

#[derive(Debug, Copy, Clone, Default)]
pub struct Rotation {
    pub rpm: u32,
    pub accel: u32,
    pub step_mode: StepMode128,
    pub duration_of_one_direction_cycle_ms: u64,
    pub steps_for_one_direction_cycle: u64,
    pub direction: Direction,
    pub pause_before_direction_change_ms: u64,
}

impl Rotation {
    pub fn new(rpm: u32, accel: u32, step_mode: StepMode128, duration_of_one_direction_cycle_ms: u64, steps_for_one_direction_cycle: u64, direction: Direction, pause_before_direction_change_ms: u64) -> Self {
        Self {
            rpm,
            accel,
            step_mode,
            duration_of_one_direction_cycle_ms,
            steps_for_one_direction_cycle,
            direction,
            pause_before_direction_change_ms,
        }
    }

    fn test_rotation() -> Self {
        Self {
            rpm: 1000,
            accel: 6000,
            step_mode: StepMode128::M8,
            duration_of_one_direction_cycle_ms: 0,
            steps_for_one_direction_cycle: 0,
            direction: Direction::Forward,
            pause_before_direction_change_ms: 1000,
        }
    }

    fn test_agitation() -> Self {
        Self {
            rpm: 2000,
            accel: 6000,
            step_mode: StepMode128::Full,
            duration_of_one_direction_cycle_ms: 4000,
            steps_for_one_direction_cycle: 0,
            direction: Direction::Forward,
            pause_before_direction_change_ms: 1000,
        }
    }


    /// Rotation to bytes for serial communication
    pub fn to_bytes(&self) -> [u8; 34] {
        let mut bytes = [0u8; 34];
        bytes[0..4].copy_from_slice(&self.rpm.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.accel.to_le_bytes());
        bytes[8..9].copy_from_slice(&self.step_mode.to_byte().to_le_bytes());
        bytes[9..17].copy_from_slice(&self.duration_of_one_direction_cycle_ms.to_le_bytes());
        bytes[17..25].copy_from_slice(&self.steps_for_one_direction_cycle.to_le_bytes());
        bytes[25..26].copy_from_slice(&self.direction.to_byte().to_le_bytes());
        bytes[26..34].copy_from_slice(&self.pause_before_direction_change_ms.to_le_bytes());
        bytes
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct Protocol {
    pub rotation: Rotation,
    pub rotation_duration_ms: u64,
    pub pause_before_agitation_ms: u64,
    pub agitation: Rotation,
    pub agitation_duration_ms: u64,
    pub pause_after_agitation_ms: u64,
    pub global_duration_ms: u64,
}


impl Protocol {
    pub fn new(rotation: Rotation, rotation_duration_ms: u64, pause_before_agitation_ms: u64, agitation: Rotation, agitation_duration_ms: u64, pause_after_agitation_ms: u64, global_duration_ms: u64) -> Self {
        Self {
            rotation,
            rotation_duration_ms,
            pause_before_agitation_ms,
            agitation,
            agitation_duration_ms,
            pause_after_agitation_ms,
            global_duration_ms,
        }
    }

    pub fn test_protocol() -> Self {
        Self {
            rotation: Rotation::test_rotation(),
            rotation_duration_ms: 10000,
            pause_before_agitation_ms: 1000,
            agitation: Rotation::test_agitation(),
            agitation_duration_ms: 10000,
            pause_after_agitation_ms: 1000,
            global_duration_ms: 100000,
        }
    }

    /// Protocol to bytes for serial communication
    pub fn bytes_vec_to_send(&self) -> Vec<u8> {
        let mut bytes = [0u8; BYTES];
        bytes[0] = b'a';
        bytes[1..35].copy_from_slice(&self.rotation.to_bytes());
        bytes[35..43].copy_from_slice(&self.rotation_duration_ms.to_le_bytes());
        bytes[43..51].copy_from_slice(&self.pause_before_agitation_ms.to_le_bytes());
        bytes[51..85].copy_from_slice(&self.agitation.to_bytes());
        bytes[85..93].copy_from_slice(&self.agitation_duration_ms.to_le_bytes());
        bytes[93..101].copy_from_slice(&self.pause_after_agitation_ms.to_le_bytes());
        bytes[101..109].copy_from_slice(&self.global_duration_ms.to_le_bytes());
        bytes[109] = b'z';
        bytes.to_vec()
    }
}