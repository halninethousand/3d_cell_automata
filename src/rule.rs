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

    /// Combine multiple rule values with OR (union of conditions)
    /// Example: "5-10, 12, 14" = from_range(5,10).or(new(&[12, 14]))
    pub fn or(self, other: Self) -> Self {
        Self {
            bitmask: self.bitmask | other.bitmask,
        }
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

    /// "Clouds 1" - Cloud-like wispy structures (13-26/13-14,17-19/2/M)
    pub fn clouds_1() -> Self {
        Self {
            survival: RuleValue::from_range(13, 26),
            birth: RuleValue::from_range(13, 14).or(RuleValue::from_range(17, 19)),
            states: 2,
            neighbor_method: NeighborMethod::Moore,
        }
    }

    /// "Amoeba" - Slowly morphing blob-like organism (9-26/5-7,12-13,15/5/M)
    pub fn amoeba() -> Self {
        Self {
            survival: RuleValue::from_range(9, 26),
            birth: RuleValue::from_range(5, 7)
                .or(RuleValue::from_range(12, 13))
                .or(RuleValue::new(&[15])),
            states: 5,
            neighbor_method: NeighborMethod::Moore,
        }
    }

    /// "Architecture" - Builds architectural-looking structures (4-6/3/2/M)
    pub fn architecture() -> Self {
        Self {
            survival: RuleValue::from_range(4, 6),
            birth: RuleValue::new(&[3]),
            states: 2,
            neighbor_method: NeighborMethod::Moore,
        }
    }

    /// "Brain" - Cellular structures resembling brain tissue (4/2/3/M)
    pub fn brain() -> Self {
        Self {
            survival: RuleValue::new(&[4]),
            birth: RuleValue::new(&[2]),
            states: 3,
            neighbor_method: NeighborMethod::Moore,
        }
    }

    /// "Builder 2" - Another builder variant (5-7/1/2/M)
    pub fn builder_2() -> Self {
        Self {
            survival: RuleValue::from_range(5, 7),
            birth: RuleValue::new(&[1]),
            states: 2,
            neighbor_method: NeighborMethod::Moore,
        }
    }

    /// "Coral" - Coral-like branching structures (5-8/6-7,9,12/8/M)
    pub fn coral() -> Self {
        Self {
            survival: RuleValue::from_range(5, 8),
            birth: RuleValue::from_range(6, 7)
                .or(RuleValue::new(&[9, 12])),
            states: 8,
            neighbor_method: NeighborMethod::Moore,
        }
    }

    /// "Crystal Growth 1" - Growing crystal formations (0-6/1,3/2/M)
    pub fn crystal_growth_1() -> Self {
        Self {
            survival: RuleValue::from_range(0, 6),
            birth: RuleValue::new(&[1, 3]),
            states: 2,
            neighbor_method: NeighborMethod::Moore,
        }
    }

    /// "Diamond Growth" - Diamond-like crystal formations (5-6/7-8/10/M)
    pub fn diamond_growth() -> Self {
        Self {
            survival: RuleValue::from_range(5, 6),
            birth: RuleValue::from_range(7, 8),
            states: 10,
            neighbor_method: NeighborMethod::Moore,
        }
    }

    /// "Pulse Waves" - Creates wave-like pulse patterns (3-8/3-7/3/M)
    pub fn pulse_waves() -> Self {
        Self {
            survival: RuleValue::from_range(3, 8),
            birth: RuleValue::from_range(3, 7),
            states: 3,
            neighbor_method: NeighborMethod::Moore,
        }
    }

    /// "Pyroclastic" - Explosive volcanic-like patterns (4-7/6-8/10/M)
    pub fn pyroclastic() -> Self {
        Self {
            survival: RuleValue::from_range(4, 7),
            birth: RuleValue::from_range(6, 8),
            states: 10,
            neighbor_method: NeighborMethod::Moore,
        }
    }

    /// "Spiky Growth" - Creates spiky protrusions (5-6/4/3/M)
    pub fn spiky_growth() -> Self {
        Self {
            survival: RuleValue::from_range(5, 6),
            birth: RuleValue::new(&[4]),
            states: 3,
            neighbor_method: NeighborMethod::Moore,
        }
    }

    /// "Shells" - Shell-like layered structures (4-5/3/3/M)
    pub fn shells() -> Self {
        Self {
            survival: RuleValue::from_range(4, 5),
            birth: RuleValue::new(&[3]),
            states: 3,
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

    /// Create a rule from ranges
    /// Example: survival 4-7, birth 6-8, 10 states, Moore
    pub fn from_ranges(
        survival_min: u8, survival_max: u8,
        birth_min: u8, birth_max: u8,
        states: u8,
        neighbor_method: NeighborMethod
    ) -> Self {
        Self {
            survival: RuleValue::from_range(survival_min, survival_max),
            birth: RuleValue::from_range(birth_min, birth_max),
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
