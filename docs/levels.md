# `src/levels.rs` — Définition des niveaux de difficulté

## Rôle du fichier

Contient la table statique des 5 niveaux de difficulté du jeu. Rien d'autre.

---

## `LevelDef`

```rust
pub struct LevelDef {
    pub subtitle: &'static str,
    pub energy_min: u32,
    pub energy_max: u32,
    pub crystal_min: u32,
    pub crystal_max: u32,
}
```

Définit uniquement ce qui varie entre niveaux : le nom affiché et les plages de quantité par gisement. Chaque gisement placé sur la carte tirera sa quantité au hasard dans `[min, max]`.

---

## `LEVELS` — tableau des 5 niveaux

| Index | Nom        | Énergie   | Cristaux  |
|-------|------------|-----------|-----------|
| 0     | Initiation | 10 – 20   | 10 – 20   |
| 1     | Normal     | 30 – 50   | 30 – 50   |
| 2     | Difficile  | 60 – 100  | 60 – 100  |
| 3     | Expert     | 120 – 200 | 120 – 200 |
| 4     | Ultra      | 250 – 500 | 250 – 500 |

**Pourquoi cette conception ?**

- **`const` statique** : les données de niveau ne changent jamais à l'exécution → `const` garantit qu'elles vivent dans le segment de données en lecture seule, zéro allocation au runtime.
- **`&'static str`** : les chaînes de noms sont des littéraux compilés, pas d'allocation `String` nécessaire.
- **Paramètre de quantité seulement** : la difficulté ne vient pas du nombre de ressources sur la carte (toujours `largeur × hauteur / 35`), mais de la quantité par gisement. En niveau Ultra, ramener un seul gisement prend beaucoup plus de voyages — les collecteurs font plus d'aller-retours, la partie est plus longue et nécessite plus d'améliorations.

**Lien avec `SaveData`** : les bonus achetés au store (`energy_bonus`, `crystal_bonus`) s'ajoutent aux valeurs du niveau dans `build_config()`. Ainsi un joueur qui a progressé dans le store voit des gisements plus riches même en niveau difficile, sans modifier les valeurs de base ici.
