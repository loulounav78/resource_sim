use crate::save::SaveData;

pub struct StoreItem {
    pub name: &'static str,
    pub description: &'static str,
    pub cost: u32,
}

pub const STORE_ITEMS: [StoreItem; 5] = [
    StoreItem { name: "Scout +1",           description: "Ajoute un robot explorateur",     cost: 50  },
    StoreItem { name: "Collector +1",       description: "Ajoute un robot collecteur",      cost: 75  },
    StoreItem { name: "Cargo +5",           description: "Augmente la capacite de cargo",   cost: 30  },
    StoreItem { name: "Depot Energie +10",  description: "Augmente les depots d'energie",   cost: 40  },
    StoreItem { name: "Depot Cristaux +10", description: "Augmente les depots de cristaux", cost: 40  },
];

/// Coût actuel d'un item selon le nombre d'achats déjà effectués.
pub fn item_cost(item_idx: usize, save: &SaveData) -> u32 {
    STORE_ITEMS[item_idx].cost * (save.upgrade_counts[item_idx] + 1)
}

/// Tente d'acheter l'item `item_idx`. Retourne true si l'achat a réussi.
pub fn apply_upgrade(item_idx: usize, save: &mut SaveData) -> bool {
    let cost = item_cost(item_idx, save);
    if save.total_crystals < cost {
        return false;
    }
    save.total_crystals -= cost;
    save.upgrade_counts[item_idx] += 1;
    match item_idx {
        0 => save.num_scouts = (save.num_scouts + 1).min(10),
        1 => save.num_collectors = (save.num_collectors + 1).min(10),
        2 => save.carry_capacity = (save.carry_capacity + 5).min(50),
        3 => save.energy_bonus += 10,
        4 => save.crystal_bonus += 10,
        _ => {}
    }
    true
}
