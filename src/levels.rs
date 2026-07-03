pub struct LevelDef {
    pub subtitle: &'static str,
    pub energy_min: u32,
    pub energy_max: u32,
    pub crystal_min: u32,
    pub crystal_max: u32,
}

pub const LEVELS: [LevelDef; 5] = [
    LevelDef { subtitle: "Initiation",  energy_min: 10,  energy_max: 20,  crystal_min: 10,  crystal_max: 20  },
    LevelDef { subtitle: "Normal",      energy_min: 30,  energy_max: 50,  crystal_min: 30,  crystal_max: 50  },
    LevelDef { subtitle: "Difficile",   energy_min: 60,  energy_max: 100, crystal_min: 60,  crystal_max: 100 },
    LevelDef { subtitle: "Expert",      energy_min: 120, energy_max: 200, crystal_min: 120, crystal_max: 200 },
    LevelDef { subtitle: "Ultra",       energy_min: 250, energy_max: 500, crystal_min: 250, crystal_max: 500 },
];
