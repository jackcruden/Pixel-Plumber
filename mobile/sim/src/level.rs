//! Authored level data (`levels/*.ron`). Authored pipe/machinery structure is
//! hard geometry written into the field directly; deposits are layered noise.
//! Each level is an independent, self-contained scene (open question Q1 —
//! do not build cross-level streaming until it is resolved).

use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct NoiseSpec {
    pub scale: f32,
    pub octaves: u32,
    /// Field is solid where fbm > threshold (fill) or carved where
    /// fbm > threshold (caves), depending on use.
    pub threshold: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Stratum {
    pub material: String,
    /// Depth band as fractions of level height, from..to.
    pub from: f32,
    pub to: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Emitter {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    /// Particles per second.
    pub rate: f32,
    pub kind: String,
    /// Emitter stops after this many particles (0 = unlimited).
    #[serde(default)]
    pub max: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MineralVein {
    pub x: f32,
    pub y: f32,
    pub r: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Level {
    pub name: String,
    pub width: usize,
    pub height: usize,
    pub seed: u64,
    pub strata: Vec<Stratum>,
    pub fill_noise: NoiseSpec,
    pub cave_noise: NoiseSpec,
    #[serde(default)]
    pub pipes: Vec<Rect>,
    #[serde(default)]
    pub clears: Vec<Rect>,
    #[serde(default)]
    pub mineral_veins: Vec<MineralVein>,
    #[serde(default)]
    pub emitters: Vec<Emitter>,
    #[serde(default)]
    pub molten_pools: Vec<Rect>,
    #[serde(default)]
    pub contaminant_pools: Vec<Rect>,
    pub intake: Rect,
    pub goal_volume: u32,
    pub goal_purity: f32,
    pub player_spawn: (f32, f32),
    #[serde(default)]
    pub ambient_heat_base: f32,
    #[serde(default)]
    pub ambient_heat_per_depth: f32,
}

impl Level {
    pub fn from_ron(src: &str) -> Result<Self, String> {
        ron::from_str(src).map_err(|e| e.to_string())
    }
}
