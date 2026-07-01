# `src/config.rs` — Configuration d'une partie

## Rôle du fichier

Regroupe tous les paramètres qui définissent une partie avant qu'elle ne commence : combien de robots, quelle capacité de cargo, quelles quantités de ressources, quelle puissance de cassage de mur. C'est l'objet de configuration passé à `start_simulation()`.

---

## `Priority` — stratégie de ciblage des collecteurs

```rust
pub enum Priority { None, Energy, Crystal }
```

Indique aux collecteurs quel type de ressource prioriser quand plusieurs gisements sont connus :

- `None` → le plus proche (distance Manhattan)
- `Energy` → le gisement d'énergie le plus proche d'abord, cristaux en fallback
- `Crystal` → l'inverse

**État actuel :** `Priority` est créé à `Priority::None` dans `build_config()` (`main.rs`), la logique de sélection est implémentée dans `robot::best_target()`, mais aucune interface ne permet encore de changer la priorité en jeu. Les `#[allow(dead_code)]` confirment que c'est prévu mais pas encore exposé. La structure est là pour accueillir cette feature sans refactoring.

---

## `Config` — snapshot des paramètres d'une partie

```rust
pub struct Config {
    pub num_scouts: usize,
    pub num_collectors: usize,
    pub priority: Priority,
    pub carry_capacity: u32,
    pub energy_min: u32,
    pub energy_max: u32,
    pub crystal_min: u32,
    pub crystal_max: u32,
    pub wall_break_power: u8,
}
```

Construit dans `main.rs::build_config()` en combinant :
- les données de `SaveData` (progression du joueur : nombre de robots, bonus achetés au store...)
- les données du `LevelDef` sélectionné (plages de quantités de ressources par niveau)

**Pourquoi un struct dédié plutôt que passer `SaveData` directement à `start_simulation` ?**  
`SaveData` contient des données persistentes (total_energy, favoris...) qui n'ont rien à faire dans la simulation. `Config` est un sous-ensemble propre et immuable des paramètres de jeu — il est cloné/passé aux threads sans embarquer l'état de sauvegarde.
