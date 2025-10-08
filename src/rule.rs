use bevy::prelude::Resource;
use bevy::math::{IVec3, ivec3};

/// Neighbor counting method
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum NeighborMethod {
    Moore,      // 26 neighbors (3x3x3 cube minus center)
    VonNeumann, // 6 neighbors (face-adjacent only)
}

impl NeighborMethod {
    pub fn get_neighbors(&self) -> &'static [IVec3] {
        match self {
            NeighborMethod::Moore => &MOORE_NEIGHBORS,
            NeighborMethod::VonNeumann => &VON_NEUMANN_NEIGHBORS,
        }
    }

    pub fn max_neighbors(&self) -> u8 {
        match self {
            NeighborMethod::Moore => 26,
            NeighborMethod::VonNeumann => 6,
        }
    }
}

/// Von Neumann neighborhood: 6 face-adjacent cells
pub static VON_NEUMANN_NEIGHBORS: [IVec3; 6] = [
    ivec3( 1,  0,  0),
    ivec3(-1,  0,  0),
    ivec3( 0,  1,  0),
    ivec3( 0, -1,  0),
    ivec3( 0,  0,  1),
    ivec3( 0,  0, -1),
];

/// Moore neighborhood: 26 surrounding cells (3x3x3 minus center)
pub static MOORE_NEIGHBORS: [IVec3; 26] = [
    // Bottom layer (z = -1)
    ivec3(-1, -1, -1),
    ivec3( 0, -1, -1),
    ivec3( 1, -1, -1),
    ivec3(-1,  0, -1),
    ivec3( 0,  0, -1),
    ivec3( 1,  0, -1),
    ivec3(-1,  1, -1),
    ivec3( 0,  1, -1),
    ivec3( 1,  1, -1),
    // Middle layer (z = 0)
    ivec3(-1, -1,  0),
    ivec3( 0, -1,  0),
    ivec3( 1, -1,  0),
    ivec3(-1,  0,  0),
    // Skip center (0, 0, 0)
    ivec3( 1,  0,  0),
    ivec3(-1,  1,  0),
    ivec3( 0,  1,  0),
    ivec3( 1,  1,  0),
    // Top layer (z = 1)
    ivec3(-1, -1,  1),
    ivec3( 0, -1,  1),
    ivec3( 1, -1,  1),
    ivec3(-1,  0,  1),
    ivec3( 0,  0,  1),
    ivec3( 1,  0,  1),
    ivec3(-1,  1,  1),
    ivec3( 0,  1,  1),
    ivec3( 1,  1,  1),
];

/// Rule value - efficient lookup table for neighbor counts using bit manipulation
/// Uses a u32 as a bitmask where bit N represents whether neighbor count N matches
/// This is more cache-friendly than a 27-element bool array (4 bytes vs 27 bytes)
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct RuleValue {
    // Bitmask where bit N is set if neighbor count N matches the rule
    // Bits 0-26 are used (27 bits total for Moore neighborhood, 0-6 for Von Neumann)
    bitmask: u32,
}

impl RuleValue {
    /// Create a rule value from specific neighbor counts
    pub fn new(counts: &[u8]) -> Self {
        let mut bitmask = 0u32;
        for &count in counts {
            if count < 27 {
                bitmask |= 1 << count;
            }
        }
        Self { bitmask }
    }

    /// Create a rule value from a range of neighbor counts
    pub fn from_range(min: u8, max: u8) -> Self {
        let mut bitmask = 0u32;
        for count in min..=max.min(26) {
            bitmask |= 1 << count;
        }
        Self { bitmask }
    }

    /// Check if a neighbor count matches this rule
    /// This is a single bit check - extremely fast!
    #[inline]
    pub fn matches(&self, count: u8) -> bool {
        if count >= 27 {
            return false;
        }
        (self.bitmask & (1 << count)) != 0
    }
}

/// Cellular automata rule definition
#[derive(Clone, PartialEq, Debug, Resource)]
pub struct Rule {
    /// Which neighbor counts keep a cell alive at max_state
    pub survival: RuleValue,
    /// Which neighbor counts spawn a new cell
    pub birth: RuleValue,
    /// Number of states (0 = dead, 1 = about to die, max_state = newly born)
    pub states: u8,
    /// Neighborhood type
    pub neighbor_method: NeighborMethod,
}

impl Rule {
    /// Create the "445" rule (4/4/5/M)
    pub fn rule_445() -> Self {
        Self {
            survival: RuleValue::new(&[4]),
            birth: RuleValue::new(&[4]),
            states: 5,
            neighbor_method: NeighborMethod::Moore,
        }
    }

    /// "Builder" - Creates complex expanding structures
    pub fn builder() -> Self {
        Self {
            survival: RuleValue::new(&[2, 6, 9]),
            birth: RuleValue::new(&[4, 6, 8, 9, 10]),
            states: 10,
            neighbor_method: NeighborMethod::Moore,
        }
    }

    /// "Fancy Snancy" - Complex chaotic patterns
    pub fn fancy_snancy() -> Self {
        Self {
            survival: RuleValue::new(&[0,1,2,3,7,8,9,11,13,18,21,22,24,26]),
            birth: RuleValue::new(&[4,13,17,20,21,22,23,24,26]),
            states: 4,
            neighbor_method: NeighborMethod::Moore,
        }
    }

    /// "Pretty Crystals" - Forms crystalline structures
    pub fn pretty_crystals() -> Self {
        Self {
            survival: RuleValue::new(&[5,6,7,8]),
            birth: RuleValue::new(&[6,7,9]),
            states: 10,
            neighbor_method: NeighborMethod::Moore,
        }
    }

    /// "Slowly Expanding Blob" - Gradually growing structure
    pub fn expanding_blob() -> Self {
        Self {
            survival: RuleValue::from_range(9, 26),
            birth: RuleValue::new(&[5,6,7,12,13,15]),
            states: 20,
            neighbor_method: NeighborMethod::Moore,
        }
    }

    /// Create a custom rule
    pub fn new(survival: &[u8], birth: &[u8], states: u8, neighbor_method: NeighborMethod) -> Self {
        Self {
            survival: RuleValue::new(survival),
            birth: RuleValue::new(birth),
            states,
            neighbor_method,
        }
    }

    /// Check if a cell should survive
    #[inline]
    pub fn should_survive(&self, neighbors: u8) -> bool {
        self.survival.matches(neighbors)
    }

    /// Check if a cell should be born
    #[inline]
    pub fn should_birth(&self, neighbors: u8) -> bool {
        self.birth.matches(neighbors)
    }
}
