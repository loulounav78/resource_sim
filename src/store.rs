use crate::save::SaveData;
use crate::simulation::wall_break_energy_cost;

#[derive(Clone, Copy, PartialEq)]
pub enum StoreCurrency {
    Crystals,
    Energy,
}

pub struct StoreItem {
    pub name: &'static str,
    pub description: &'static str,
    pub cost: u32,
    pub currency: StoreCurrency,
}

pub enum PurchaseResult {
    Bought,
    InsufficientResources,
    MaxedOut,
}

pub const WALL_BREAK_ITEM_IDX: usize = 5;

pub const STORE_ITEMS: [StoreItem; 6] = [
    StoreItem {
        name: "Scout +1",
        description: "Ajoute un robot explorateur",
        cost: 50,
        currency: StoreCurrency::Crystals,
    },
    StoreItem {
        name: "Collector +1",
        description: "Ajoute un robot collecteur",
        cost: 75,
        currency: StoreCurrency::Crystals,
    },
    StoreItem {
        name: "Cargo +5",
        description: "Augmente la capacite de cargo",
        cost: 30,
        currency: StoreCurrency::Crystals,
    },
    StoreItem {
        name: "Depot Energie +10",
        description: "Augmente les depots d'energie",
        cost: 40,
        currency: StoreCurrency::Crystals,
    },
    StoreItem {
        name: "Depot Cristaux +10",
        description: "Augmente les depots de cristaux",
        cost: 40,
        currency: StoreCurrency::Crystals,
    },
    StoreItem {
        name: "Laser murs +1",
        description: "Boost special cassage des murs",
        cost: 50,
        currency: StoreCurrency::Energy,
    },
];

pub fn item_cost(item_idx: usize, save: &SaveData) -> u32 {
    if item_idx == WALL_BREAK_ITEM_IDX {
        return wall_break_energy_cost((save.wall_break_power + 1).min(3));
    }

    STORE_ITEMS[item_idx].cost * (upgrade_count(save, item_idx) + 1)
}

pub fn can_afford(item_idx: usize, save: &SaveData) -> bool {
    let cost = item_cost(item_idx, save);
    match STORE_ITEMS[item_idx].currency {
        StoreCurrency::Crystals => save.total_crystals >= cost,
        StoreCurrency::Energy => save.total_energy >= cost,
    }
}

pub fn is_maxed(item_idx: usize, save: &SaveData) -> bool {
    match item_idx {
        0 => save.num_scouts >= 10,
        1 => save.num_collectors >= 10,
        2 => save.carry_capacity >= 50,
        WALL_BREAK_ITEM_IDX => save.wall_break_power >= 3,
        _ => false,
    }
}

pub fn apply_upgrade(item_idx: usize, save: &mut SaveData) -> PurchaseResult {
    if is_maxed(item_idx, save) {
        return PurchaseResult::MaxedOut;
    }

    let cost = item_cost(item_idx, save);
    match STORE_ITEMS[item_idx].currency {
        StoreCurrency::Crystals => {
            if save.total_crystals < cost {
                return PurchaseResult::InsufficientResources;
            }
            save.total_crystals -= cost;
        }
        StoreCurrency::Energy => {
            if save.total_energy < cost {
                return PurchaseResult::InsufficientResources;
            }
            save.total_energy -= cost;
        }
    }

    increment_upgrade_count(save, item_idx);
    match item_idx {
        0 => save.num_scouts = (save.num_scouts + 1).min(10),
        1 => save.num_collectors = (save.num_collectors + 1).min(10),
        2 => save.carry_capacity = (save.carry_capacity + 5).min(50),
        3 => save.energy_bonus += 10,
        4 => save.crystal_bonus += 10,
        WALL_BREAK_ITEM_IDX => save.wall_break_power = (save.wall_break_power + 1).min(3),
        _ => {}
    }
    PurchaseResult::Bought
}

fn upgrade_count(save: &SaveData, item_idx: usize) -> u32 {
    save.upgrade_counts.get(item_idx).copied().unwrap_or(0)
}

fn increment_upgrade_count(save: &mut SaveData, item_idx: usize) {
    if save.upgrade_counts.len() <= item_idx {
        save.upgrade_counts.resize(item_idx + 1, 0);
    }
    save.upgrade_counts[item_idx] += 1;
}
