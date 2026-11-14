use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyStage {
    pub stage_name: String,
    pub target_energy: f64,
    pub duration_mins: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnergyCurveType {
    Steady,
    BuildUp,
    PeakParty,
    Waves,
}

