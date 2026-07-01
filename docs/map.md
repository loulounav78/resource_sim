# `src/map.rs` — Génération procédurale de la carte

## Rôle du fichier

Définit la structure de la carte (`Map` + `Tile`) et génère procéduralement le terrain + les ressources à partir d'une seed.

---

## `Tile` — type de case

```rust
pub enum Tile {
    Empty,
    Obstacle(u8),  // u8 = durabilité restante (1 à 3)
    Base,
}
```

`Obstacle(u8)` encode à la fois la nature de la case **et** sa durabilité restante dans la même valeur. Pas besoin d'un champ séparé — le `match` sur la variante suffit pour afficher la bonne couleur (`O` / `Ø` / `¤` dans `ui.rs`) et gérer le cassage progressif dans `damage_wall()`.

`WALL_MAX_DURABILITY = 3` : les murs naissent toujours à durabilité 3.

---

## `mix_seed` — dispersion de seed (SplitMix64)

```rust
fn mix_seed(seed: u32) -> u64 { ... }
```

Applique la fonction de hachage **SplitMix64** à la seed brute avant de l'utiliser pour le bruit de Perlin.

**Pourquoi ?** La table de permutation interne de `noise::Perlin` ne disperse pas bien les petites valeurs entières : les seeds `1`, `2`, `3` produisaient des cartes visuellement quasi identiques. SplitMix64 est une fonction de hachage reconnue pour casser ces corrélations — même une seed à un seul chiffre tapée à la main donne un terrain distinct.

---

## `Map::generate` — génération en 3 étapes

### 1. Terrain (bruit de Perlin)

```rust
let nx = x as f64 / width as f64 * 6.0;
let ny = y as f64 / height as f64 * 6.0;
if perlin.get([nx, ny]) > 0.22 { tiles[y][x] = Tile::Obstacle(3); }
```

Le facteur `6.0` contrôle la "fréquence" du bruit : plus il est grand, plus les blocs sont petits et fragmentés. `0.22` est le seuil de densité : environ 30 % des cases deviennent des obstacles selon la distribution du bruit de Perlin.

### 2. Zone de base (5×5 au centre)

Une zone `5×5` centrée sur `(width/2, height/2)` est forcée en `Tile::Base`. Cela garantit que les robots et le joueur ont toujours de l'espace autour de leur point de départ, quelle que soit la seed.

### 3. Ressources (RNG déterministe)

```rust
let mut rng = StdRng::seed_from_u64(mixed ^ 0xA5A5_A5A5_A5A5_A5A5);
let target = (width * height) / 35;
```

Un `StdRng` séparé (mais dérivé de la même seed mixée) place les gisements aléatoirement sur les cases `Empty`. Le XOR avec `0xA5...` évite que terrain et ressources partagent exactement la même séquence RNG, ce qui produirait des artefacts.

Le quota `surface / 35` donne environ 137 gisements pour une carte 120×40 — suffisamment dense pour être jouable mais pas trivial.

**Reproductibilité** : terrain et ressources étant tous deux dérivés de la même seed, une carte favorite se recharge à l'identique à chaque lancement.

---

## Méthodes utilitaires

| Méthode | Ce qu'elle fait |
|---|---|
| `is_passable(x, y)` | Vrai si la case existe et n'est pas un obstacle |
| `is_base(x, y)` | Vrai si la case est de type `Base` |
| `damage_wall(x, y, dmg)` | Réduit la durabilité du mur, le détruit si ≤ 0 |
| `base_pos()` | Renvoie `(width/2, height/2)` — centre de la carte |

`damage_wall` retourne `bool` pour que `SimState::damage_wall_from_player` sache si le coût en énergie doit être prélevé (on ne paie que si un mur a effectivement été touché).
