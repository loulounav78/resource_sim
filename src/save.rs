use serde::{Deserialize, Serialize};

const SAVE_FILE: &str = "save.json";
const UPGRADE_SLOT_COUNT: usize = 6;

/// Une carte favorite, identifiee par la seed utilisee pour la generer
/// (le niveau permet de reconstruire la config de jeu lors du relancement).
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct FavoriteMap {
    pub seed: u32,
    pub level: usize,
}

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
    /// Puissance du cassage de mur du vaisseau joueur (1 a 3 etats par action).
    #[serde(default = "default_wall_break_power")]
    pub wall_break_power: u8,
    /// Nombre d'achats par item du store (pour l'escalade des prix)
    #[serde(default = "default_upgrade_counts")]
    pub upgrade_counts: Vec<u32>,
    /// Cartes mises en favori (rejouables tant qu'elles restent dans la liste)
    #[serde(default)]
    pub favorite_maps: Vec<FavoriteMap>,
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
            wall_break_power: default_wall_break_power(),
            upgrade_counts: default_upgrade_counts(),
            favorite_maps: Vec::new(),
        }
    }
}

impl SaveData {
    pub fn normalize(&mut self) {
        self.wall_break_power = self.wall_break_power.clamp(1, 3);
        if self.upgrade_counts.len() < UPGRADE_SLOT_COUNT {
            self.upgrade_counts.resize(UPGRADE_SLOT_COUNT, 0);
        }
    }

    pub fn is_favorite(&self, seed: u32) -> bool {
        self.favorite_maps.iter().any(|f| f.seed == seed)
    }

    /// Ajoute ou retire la carte des favoris. Renvoie true si elle vient d'etre ajoutee.
    pub fn toggle_favorite(&mut self, level: usize, seed: u32) -> bool {
        if let Some(pos) = self.favorite_maps.iter().position(|f| f.seed == seed) {
            self.favorite_maps.remove(pos);
            false
        } else {
            self.favorite_maps.push(FavoriteMap { seed, level });
            true
        }
    }
}

pub fn load() -> SaveData {
    let mut data: SaveData = std::fs::read_to_string(SAVE_FILE)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    data.normalize();
    data
}

pub fn persist(data: &SaveData) {
    if let Ok(json) = serde_json::to_string_pretty(data) {
        let _ = std::fs::write(SAVE_FILE, json);
    }
}

fn default_wall_break_power() -> u8 {
    1
}

fn default_upgrade_counts() -> Vec<u32> {
    vec![0; UPGRADE_SLOT_COUNT]
}
