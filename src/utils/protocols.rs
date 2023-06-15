use serde::{Deserialize, Serialize};
use stepgen_new::x64::Stepgen;

use crate::app::{BYTES, MAX_RPM};
use crate::utils::enums::{Direction, StepMode128};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Rotation {
    pub rpm: u32,
    pub acceleration: u32,
    pub step_mode: StepMode128,
    pub duration_of_one_direction_cycle_ms: u64,
    pub steps_for_one_direction_cycle: u64,
    pub direction: Direction,
    pub pause_before_direction_change_ms: u64,
}

impl Default for Rotation {
    fn default() -> Self {
        Self {
            rpm: 1,
            acceleration: 1,
            step_mode: StepMode128::Full,
            duration_of_one_direction_cycle_ms: 0,
            steps_for_one_direction_cycle: 0,
            direction: Direction::Forward,
            pause_before_direction_change_ms: 0,
        }
    }
}

impl Rotation {
    pub fn get_min_duration(&self) -> u64 {
        self.duration_of_one_direction_cycle_ms + self.pause_before_direction_change_ms
    }

    pub fn max_rpm_for_stepmode(&self) -> u32 {
        match self.step_mode {
            StepMode128::Full => MAX_RPM,
            StepMode128::M2 => MAX_RPM / 2,
            StepMode128::M4 => MAX_RPM / 4,
            StepMode128::M8 => MAX_RPM / 8,
            StepMode128::M16 => MAX_RPM / 16,
            StepMode128::M32 => MAX_RPM / 32,
            StepMode128::M64 => MAX_RPM / 64,
            StepMode128::M128 => MAX_RPM / 128,
        }
    }


    pub fn create_stepgen(&self) -> stepgen_new::x64::Stepgen<1_000_000> {
        let target_rpm = self.rpm * self.step_mode.get_multiplier();
        let target_accel = self.acceleration * self.step_mode.get_multiplier();
        Stepgen::new(target_rpm, target_accel,
                     self.steps_for_one_direction_cycle, self.duration_of_one_direction_cycle_ms).unwrap()
    }


    /// Rotation to bytes for serial communication
    pub fn convert_to_bytes(&self) -> [u8; 34] {
        let mut bytes = [0u8; 34];
        bytes[0..4].copy_from_slice(&self.rpm.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.acceleration.to_le_bytes());
        bytes[8..9].copy_from_slice(self.step_mode.convert_to_bytes_slice());
        bytes[9..17].copy_from_slice(&self.duration_of_one_direction_cycle_ms.to_le_bytes());
        bytes[17..25].copy_from_slice(&self.steps_for_one_direction_cycle.to_le_bytes());
        bytes[25..26].copy_from_slice(self.direction.convert_to_byte_slice());
        bytes[26..34].copy_from_slice(&self.pause_before_direction_change_ms.to_le_bytes());
        bytes
    }
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize)]
pub struct Protocol {
    pub rotation: Rotation,
    pub rotation_duration_ms: u64,
    pub pause_pre_agitation_ms: u64,
    pub agitation: Rotation,
    pub agitation_duration_ms: u64,
    pub pause_post_agitation_ms: u64,
    pub global_duration_ms: u64,
}


impl Protocol {
    pub fn get_duration_without_pause(&self) -> u64 {
        self.rotation_duration_ms + self.agitation_duration_ms
    }

    /// Protocol to bytes for serial communication
    pub fn protocol_as_bytes(&self) -> [u8; BYTES] {
        let mut bytes = [0u8; BYTES];
        bytes[0] = b'a';
        bytes[1..35].copy_from_slice(&self.rotation.convert_to_bytes());
        bytes[35..43].copy_from_slice(&self.rotation_duration_ms.to_le_bytes());
        bytes[43..51].copy_from_slice(&self.pause_pre_agitation_ms.to_le_bytes());
        bytes[51..85].copy_from_slice(&self.agitation.convert_to_bytes());
        bytes[85..93].copy_from_slice(&self.agitation_duration_ms.to_le_bytes());
        bytes[93..101].copy_from_slice(&self.pause_post_agitation_ms.to_le_bytes());
        bytes[101..109].copy_from_slice(&self.global_duration_ms.to_le_bytes());
        bytes[109] = b'z';
        bytes
    }
}