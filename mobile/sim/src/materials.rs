//! Materials are data (`materials/*.ron`), not match arms. Simulation rules
//! dispatch off this table; adding a material must never require touching
//! simulation code.

use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum MaterialKind {
    Terrain,
    Fluid,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Material {
    pub id: String,
    pub kind: MaterialKind,
    #[serde(default)]
    pub hardness: f32,
    #[serde(default)]
    pub density: f32,
    #[serde(default)]
    pub thermal_conductivity: f32,
    #[serde(default)]
    pub melting_point: Option<f32>,
    #[serde(default)]
    pub value: i32,
    #[serde(default)]
    pub tool_tier_required: u32,
    pub colour: (f32, f32, f32),
    /// Fluids only: relative particle mass (stratification comes from this).
    #[serde(default = "one")]
    pub particle_mass: f32,
    /// Fluids only: extra velocity smoothing (viscosity), 0..1.
    #[serde(default)]
    pub viscosity: f32,
    /// Fluids only: heat emitted into the temperature field per step.
    #[serde(default)]
    pub heat_emission: f32,
    /// Terrain only: excavation is impossible regardless of tool tier.
    #[serde(default)]
    pub indestructible: bool,
}

fn one() -> f32 {
    1.0
}

#[derive(Clone, Debug, Deserialize)]
pub struct MaterialTable {
    pub materials: Vec<Material>,
}

impl MaterialTable {
    pub fn from_ron(src: &str) -> Result<Self, String> {
        ron::from_str(src).map_err(|e| e.to_string())
    }

    /// Index of a material by id. Level data and interaction rules resolve ids
    /// once at load time; per-cell state stores the index as `u8`.
    pub fn index_of(&self, id: &str) -> Option<u8> {
        self.materials.iter().position(|m| m.id == id).map(|i| i as u8)
    }

    pub fn get(&self, idx: u8) -> &Material {
        &self.materials[idx as usize]
    }
}
