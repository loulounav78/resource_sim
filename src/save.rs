use serde::{Deserialize, Serialize};

const SAVE_FILE: &str = "save.json";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SaveData {
    pub total_energy: u32,
    pub total_crystals: u32,
    pub num_scouts: usize,
    pub num_collectors: usize,
    pub carry_capacity: u32,
    /// Bonus additionnel sur la quantité min/max des dépots d'énergie
    pub energy_bonus: u32,
    /// Bonus additionnel sur la quantité min/max des dépots de cristaux
    pub crystal_bonus: u32,
    /// Nombre d'achats par item du store (pour l'escalade des prix)
    #[serde(default)]
    pub upgrade_counts: [u32; 5],
}

impl Default for SaveData {
    fn default() -> Self {
        Self {
            total_energy: 0,
            total_crystals: 0,
            num_scouts: 2,
            num_collectors: 2,
            carry_capacity: 5,
            energy_bonus: 0,
            crystal_bonus: 0,
            upgrade_counts: [0; 5],
        }
    }
}

pub fn load() -> SaveData {
    std::fs::read_to_string(SAVE_FILE)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn persist(data: &SaveData) {
    if let Ok(json) = serde_json::to_string_pretty(data) {
        let _ = std::fs::write(SAVE_FILE, json);
    }
}
