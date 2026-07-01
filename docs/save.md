# `src/save.rs` — Persistance de la progression

## Rôle du fichier

Gère la sauvegarde et le chargement de la progression du joueur dans `save.json`. Tout ce qui doit survivre entre deux lancements du jeu passe par ici.

---

## `FavoriteMap` — une carte favorite

```rust
pub struct FavoriteMap {
    pub seed: u32,
    pub level: usize,
}
```

Une carte est entièrement définie par sa seed (qui détermine terrain et ressources de façon déterministe) et son niveau (qui fournit les plages de quantité de ressources). Ces deux informations suffisent à reproduire une partie à l'identique.

---

## `SaveData` — données persistées

| Champ | Valeur par défaut | Rôle |
|---|---|---|
| `total_energy` | 0 | Monnaie (énergie) accumulée |
| `total_crystals` | 0 | Monnaie (cristaux) accumulée |
| `num_scouts` | 2 | Nombre de scouts (upgradable jusqu'à 10) |
| `num_collectors` | 2 | Nombre de collecteurs (upgradable jusqu'à 10) |
| `carry_capacity` | 5 | Cargo par collecteur (upgradable jusqu'à 50) |
| `energy_bonus` | 0 | Bonus de quantité ajouté aux gisements d'énergie |
| `crystal_bonus` | 0 | Bonus de quantité ajouté aux gisements de cristaux |
| `wall_break_power` | 1 | Puissance laser du joueur (1–3) |
| `upgrade_counts` | `[0; 6]` | Nombre d'achats par item du store (pour l'escalade des prix) |
| `favorite_maps` | `[]` | Liste des cartes mises en favori |

### `#[serde(default)]` — compatibilité ascendante

`wall_break_power`, `upgrade_counts` et `favorite_maps` utilisent `#[serde(default)]`. Si un `save.json` ancien (avant l'ajout de ces champs) est chargé, serde utilise la valeur par défaut au lieu d'échouer. Cela évite de perdre la progression d'un joueur après une mise à jour du jeu.

---

## `normalize()` — assainissement au chargement

```rust
pub fn normalize(&mut self) {
    self.wall_break_power = self.wall_break_power.clamp(1, 3);
    if self.upgrade_counts.len() < UPGRADE_SLOT_COUNT {
        self.upgrade_counts.resize(UPGRADE_SLOT_COUNT, 0);
    }
}
```

Corrige un éventuel `save.json` corrompu ou édité à la main. Appelé systématiquement après désérialisation dans `load()`.

---

## `load()` et `persist()`

```rust
pub fn load() -> SaveData {
    std::fs::read_to_string("save.json")
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}
```

Chaîne de `Option` : si le fichier n'existe pas **ou** si le JSON est invalide → `SaveData::default()` (partie neuve). Pas de panic, pas de message d'erreur — un joueur qui lance pour la première fois part juste avec les valeurs par défaut.

```rust
pub fn persist(data: &SaveData) {
    if let Ok(json) = serde_json::to_string_pretty(data) {
        let _ = std::fs::write("save.json", json);
    }
}
```

`let _` : les erreurs d'écriture (disque plein, permissions...) sont ignorées silencieusement. Pour un jeu TUI solo sans mode multijoueur ni anticheat, c'est acceptable — perdre une sauvegarde dans un cas extrême est moins grave que de faire crasher le jeu.

---

## `toggle_favorite(level, seed)`

```rust
pub fn toggle_favorite(&mut self, level: usize, seed: u32) -> bool {
    if let Some(pos) = self.favorite_maps.iter().position(|f| f.seed == seed) {
        self.favorite_maps.remove(pos);  // déjà favorite → retirer
        false
    } else {
        self.favorite_maps.push(FavoriteMap { seed, level });  // ajouter
        true
    }
}
```

La seed seule suffit à identifier la carte (deux niveaux avec la même seed donneraient une carte différente à cause des quantités de ressources, mais la seed est unique dans les favoris). Le niveau est stocké pour pouvoir reconstruire la config exacte au relancement.
