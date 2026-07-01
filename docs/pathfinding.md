# `src/pathfinding.rs` — Recherche de chemin (BFS)

## Rôle du fichier

Fournit deux fonctions utilisées par les threads robots pour naviguer sur la carte : un BFS complet et un calcul de voisins accessibles.

---

## `bfs(map, start, goal) -> Option<Vec<Pos>>`

Parcours en largeur (*Breadth-First Search*) sur la grille. Retourne la liste ordonnée des cases à traverser de `start` (exclu) à `goal` (inclus), ou `None` si `goal` est inaccessible.

### Fonctionnement

```
file d'attente : [start]
visited : HashMap<Pos, Pos> (case → case parente, pour reconstruire le chemin)

Pour chaque case courante :
  → si c'est le goal : reconstituer le chemin en remontant les parents
  → sinon : ajouter les voisins passables non encore visités
```

La reconstruction parcourt `visited` en remontant de `goal` jusqu'à `start`, puis inverse le résultat.

### Choix de BFS plutôt qu'A*

BFS garantit le **chemin le plus court en nombre de cases** sur une grille non pondérée. A* serait plus rapide avec une bonne heuristique (distance euclidienne / Manhattan), mais dans ce projet :
- La carte fait 120×40 = 4 800 cases au maximum — le BFS est suffisamment rapide.
- La topologie change (murs cassés par le joueur) — une heuristique ne changerait pas fondamentalement la complexité.
- La simplicité de BFS est plus lisible et moins risquée à déboguer que A*.

### Cas particulier : `next == goal`

```rust
if !visited.contains_key(&next) && (map.is_passable(next.0, next.1) || next == goal)
```

Le goal lui-même n'est pas forcément passable (ex: la base est `Tile::Base`, pas `Tile::Empty`). Cette exception permet d'y naviguer quand même.

### Retour vide vs `None`

- `Some(vec![])` : `start == goal`, déjà sur place.
- `None` : chemin impossible (zone isolée par des murs).

Les appelants (`robot.rs`) traitent `None` avec `.unwrap_or_default()` → chemin vide, le robot reste sur place ce tick.

---

## `passable_neighbors(map, pos) -> Vec<Pos>`

Retourne les 4 voisins cardinaux de `pos` qui sont passables (dans les limites de la carte et non obstacles). Utilisé par :
- Les **scouts** pour leur marche aléatoire pondérée (sans BFS).
- La fonction **`step_aside`** (`robot.rs`) pour trouver une alternative quand la case suivante du chemin est occupée.
