# `src/types.rs` — Types partagés fondamentaux

## Rôle du fichier

Définit les briques de base réutilisées dans tout le projet : alias de position, directions, et le type de ressource. Aucune logique métier ici — juste des types.

---

## `Pos` — alias de position

```rust
pub type Pos = (usize, usize);
```

Un simple alias pour `(usize, usize)` représentant une case de la carte `(x, y)`.

**Pourquoi un alias et non une struct ?**  
Un alias est zéro-coût (pas d'encapsulation supplémentaire) et permet d'utiliser `Pos` directement comme clé dans `HashMap<Pos, _>` sans implémenter `Hash` / `Eq` manuellement — `(usize, usize)` les implémente déjà. Si le projet avait besoin de coordonnées flottantes ou de méthodes (distance, addition...), une `struct` aurait été justifiée.

---

## `Direction` — déplacement cardinal

```rust
pub enum Direction { Up, Down, Left, Right }
```

Avec la méthode `delta() -> (i32, i32)` qui convertit une direction en vecteur déplacement :

| Direction | delta     |
|-----------|-----------|
| Up        | `(0, -1)` |
| Down      | `(0, 1)`  |
| Left      | `(-1, 0)` |
| Right     | `(1, 0)`  |

**Pourquoi `i32` et non `usize` ?**  
Le déplacement peut soustraire 1 à une coordonnée. Si la coordonnée est 0, `0usize - 1` panique (overflow). Avec `i32` on effectue le calcul signé, puis on vérifie `>= 0` avant de convertir en `usize`. C'est le pattern utilisé partout dans `robot.rs` et `map.rs`.

---

## `ResourceKind` — type de ressource

```rust
pub enum ResourceKind { Energy, Crystal }
```

`derive(Clone, PartialEq, Debug)` : clonable car transmis dans les `Message`, comparable (`==`) pour filtrer par type dans la logique de priorité des collecteurs, `Debug` pour le développement.

---

## `Resource` — dépôt sur la carte

```rust
pub struct Resource {
    pub kind: ResourceKind,
    pub quantity: u32,
}
```

Représente un gisement placé sur une case de la carte. `quantity` est décrémenté à chaque tick où un collecteur est dessus — quand il atteint 0 le gisement est retiré de `SimState::resources`.
