# `src/store.rs` — Boutique d'améliorations

## Rôle du fichier

Définit le catalogue d'améliorations disponibles entre deux parties et la logique d'achat/plafonnement.

---

## `STORE_ITEMS` — catalogue statique

6 items fixes, indexés de 0 à 5 :

| Idx | Nom            | Coût base | Devise   | Effet sur `SaveData` |
|-----|----------------|-----------|----------|----------------------|
| 0   | Scout +1       | 50        | Cristaux | `num_scouts += 1` (max 10) |
| 1   | Collector +1   | 75        | Cristaux | `num_collectors += 1` (max 10) |
| 2   | Cargo +5       | 30        | Cristaux | `carry_capacity += 5` (max 50) |
| 3   | Dépôt Énergie  | 40        | Cristaux | `energy_bonus += 10` |
| 4   | Dépôt Cristaux | 40        | Cristaux | `crystal_bonus += 10` |
| 5   | Laser murs +1  | variable  | Énergie  | `wall_break_power += 1` (max 3) |

**Deux devises** : la plupart des items coûtent des **cristaux** (ressource la plus précieuse, plus rare que l'énergie). Le laser coûte de l'**énergie** — cohérent thématiquement (une arme énergétique) et économiquement (ça évite que toutes les ressources ne servent qu'à une seule chose).

---

## Escalade des prix — `item_cost`

```rust
pub fn item_cost(item_idx: usize, save: &SaveData) -> u32 {
    if item_idx == WALL_BREAK_ITEM_IDX {
        return wall_break_energy_cost((save.wall_break_power + 1).min(3));
    }
    STORE_ITEMS[item_idx].cost * (upgrade_count(save, item_idx) + 1)
}
```

Pour les items 0–4 : `coût = base × (achats_déjà_faits + 1)`. Le 1er achat coûte `base × 1`, le 2e `base × 2`, etc.

**Exemple pour Scout +1 (coût base 50 cristaux) :**
- 1er scout → 50 cristaux
- 2e scout → 100 cristaux
- 3e scout → 150 cristaux
- ...jusqu'au 8e scout possible (10 - 2 de départ) → 400 cristaux

Cela garantit que le joueur investit progressivement et reste en jeu longtemps plutôt qu'acheter tout rapidement.

**Laser** : prix fixe par palier (50 → 150 → 350 énergie), correspondant exactement au coût d'**utilisation** du niveau suivant — c'est `wall_break_energy_cost(niveau_actuel + 1)`. Le joueur paye une fois pour débloquer, puis paye à chaque usage.

---

## `is_maxed` — plafonnement

| Item | Plafond |
|---|---|
| Scout | 10 robots |
| Collector | 10 robots |
| Cargo | 50 unités |
| Laser | niveau 3 |
| Dépôt énergie / cristaux | aucun plafond défini (false) |

Énergie et cristaux n'ont pas de max — le joueur peut en théorie pousser les gisements à l'infini, ce qui rend les niveaux hauts toujours plus rapides.

---

## `apply_upgrade` — logique d'achat

Vérifie dans l'ordre :
1. `is_maxed` → `MaxedOut`
2. solde suffisant → `InsufficientResources`
3. Déduit le coût, incrémente `upgrade_counts[item_idx]`, applique l'effet.

`upgrade_counts` est le seul endroit où le nombre total d'achats par item est mémorisé — il sert exclusivement au calcul du prix escaladé. Séparer ce compteur de l'effet réel (`num_scouts`, etc.) permet de calculer le prix du prochain achat indépendamment de la valeur courante.
