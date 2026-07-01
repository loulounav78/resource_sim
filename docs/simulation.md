# `src/simulation.rs` — État de la simulation et actions du joueur

## Rôle du fichier

Définit `SimState`, la structure centrale partagée entre tous les threads, ainsi que les actions que le joueur peut effectuer (déplacer son vaisseau, casser des murs). C'est le cœur de l'état en jeu.

---

## Structures de données

### `RobotDisplay` — vue rendu d'un robot

```rust
pub struct RobotDisplay { pub x, pub y, pub kind: RobotKind, pub carrying: bool }
```

Contient uniquement ce que l'UI a besoin pour afficher un robot : sa position et son état visuel. C'est intentionnellement minimal — la logique de déplacement vit dans `robot.rs`, l'affichage dans `ui.rs`. Chaque thread robot met à jour sa propre entrée dans `SimState::robots[robot_idx]`.

### `BaseKnowledge` — ce que la base sait

```rust
pub struct BaseKnowledge {
    pub resources: HashMap<Pos, (ResourceKind, u32)>,
}
```

Carte des ressources **connues** — celles que les scouts ont découvertes et signalées. Distincte de `SimState::resources` qui représente les ressources **réellement présentes** sur la carte. Les collecteurs naviguent vers `knowledge.resources` ; si une ressource a été prélevée entre-temps, ils gèrent le cas à l'arrivée.

### `SimState` — état global de la simulation

| Champ | Rôle |
|---|---|
| `map` | La carte (terrain + murs) |
| `resources` | Gisements encore présents sur la carte (vérité terrain) |
| `knowledge` | Ce que les scouts ont rapporté à la base |
| `robots` | Position et état visuel de chaque robot |
| `player` | Position du vaisseau joueur |
| `scout_rally_pos` | Point de ralliement actif (`Some(pos)`) ou inactif (`None`) |
| `wall_break_power` | Puissance de cassage (1–3, upgradable au store) |
| `available_energy` | Énergie disponible pour les actions joueur |
| `collected_energy` | Total énergie récoltée (pour stats) |
| `collected_crystals` | Total cristaux récoltés (pour victoire + stats) |
| `reserved` | Cases qu'un collecteur s'est "réservées" pour éviter les conflits |

**`reserved`** : quand un collecteur choisit un gisement cible, il l'insère dans `reserved`. Les autres collecteurs l'ignorent dans `best_target()`. À la fin de la collecte ou si le gisement disparaît, il retire sa réservation. Cela évite que tous les collecteurs foncent vers la même ressource.

---

## Actions du joueur

### `move_player(direction)`

Déplace le vaisseau d'une case dans la direction donnée. Vérifie :
1. La case cible est dans les limites de la carte.
2. La case est passable (pas un mur).
3. Aucun robot n'occupe la case (anti-collision joueur/robot).

Si un ralliement scout est actif, le point de ralliement suit le joueur automatiquement.

### `damage_wall_from_player(direction)`

Inflige `wall_break_power` points de dégâts au mur adjacent dans la direction donnée. Coûte de l'énergie (voir `wall_break_energy_cost`). Ne fait rien si :
- Pas assez d'énergie disponible.
- La case visée n'est pas un obstacle.

### `set_scout_rally_active(active)`

Active ou désactive le ralliement des scouts vers la position du joueur. À l'activation :
- Prélève un coût en énergie proportionnel au nombre de scouts **non déjà sur la position du joueur** (`SCOUT_RALLY_ENERGY_COST_PER_SCOUT × n`).
- Si pas assez d'énergie, n'active pas le ralliement (fail silencieux).

---

## Coûts en énergie

```rust
pub fn wall_break_energy_cost(level: u8) -> u32 {
    match level { 0 => 0, 1 => 50, 2 => 150, _ => 350 }
}
```

Courbe exponentielle : casser un mur avec le laser niveau 3 coûte 7× plus que niveau 1. Ça équilibre le store — upgrader le laser est puissant mais "cher à l'usage".

---

## Position du joueur au spawn

`player_spawn_pos()` cherche une case passable parmi une liste d'offsets autour du centre de la base (la base est `Tile::Base`, pas passable pour le joueur). Fallback sur `base_pos()` si toutes les cases adjacentes seraient des murs — théoriquement impossible avec la zone 5×5 nettoyée par `Map::generate`.
