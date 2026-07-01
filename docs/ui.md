# `src/ui.rs` — Interface utilisateur (TUI)

## Rôle du fichier

Contient toutes les fonctions de rendu de l'interface. Aucune logique métier ici — uniquement la traduction de l'état du programme en caractères affichés dans le terminal via **ratatui**.

---

## Pattern général ratatui

Chaque fonction de dessin reçoit un `&mut Frame` (la "toile" du tick courant) et découpe l'espace en zones via `Layout` :

```rust
let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Length(3), Constraint::Min(1), ...])
    .split(area);
```

- `Constraint::Length(n)` → taille fixe en lignes/colonnes
- `Constraint::Min(n)` → prend tout l'espace restant (≥ n)
- `Constraint::Percentage(p)` → fraction de l'espace parent

Le texte est composé de `Span` (fragment de texte avec style) groupés en `Line`, groupés en `Paragraph`.

---

## `draw_main_menu`

8 zones verticales :
1. Barre de ressources (énergie + cristaux)
2. Label "Sélectionnez un niveau"
3. Cartes de niveaux — 5 colonnes égales (`Percentage(20)` × 5)
4. Bouton Store
5. Champ de saisie de seed
6. Liste des cartes favorites (hauteur dynamique : `min(6, nb_favs).max(1) + 2 bordures`)
7. Espace flexible
8. Footer (raccourcis clavier)

**Focus visuel** : la bordure passe en jaune (`Color::Yellow`) pour indiquer la zone active, gris sombre (`Color::DarkGray`) sinon. Les cartes de niveau non sélectionnées ont leur texte atténué ; la sélectionnée affiche `[ Enter ]` en fond jaune.

**Champ seed** : affiche un curseur `_` simulé quand le focus est actif. Valide en temps réel si le texte est parseable en `u32` (affiche alors `[Enter] Lancer` en vert).

---

## `draw_store`

7 zones verticales. Points notables :

- **Escalade de prix visible** : `item_cost(i, save)` est appelé à chaque rendu — le prix affiché reflète toujours le nombre d'achats déjà faits.
- **Couleur du coût** : vert si abordable, rouge si insuffisant, gris + "MAX" si maxé.
- **Largeur fixe des colonnes** : `format!("{:<22}", item.name)` et `format!("{:<38}", description)` alignent les colonnes avec padding à droite — rendu tabulaire sans widget de table.
- **Feedback** : une ligne de retour (vert = succès, rouge = erreur) s'affiche après un achat. Effacée automatiquement dès que l'utilisateur navigue vers un autre item.

---

## `draw_simulation`

Découpe l'écran en :
- Zone carte (`Constraint::Min(5)`) — la majorité de l'écran
- Barre de stats (`Constraint::Length(4)`) — en bas

### `draw_map` — rendu case par case

Parcourt `y` puis `x` dans les limites de l'écran visible (`vis_w × vis_h`). Pour chaque case, priorité d'affichage :

1. **Joueur** (`V` jaune gras) — toujours au-dessus
2. **Robot** (`x` rouge = scout, `o` magenta = collecteur vide, `@` jaune = collecteur chargé)
3. **Ressource** (`E` vert = énergie, `C` magenta clair = cristaux)
4. **Terrain** (`O` cyan clair = mur dur 3/3, `Ø` cyan = mur endommagé 2/3, `¤` bleu = mur très endommagé 1/3, `#` vert clair = base, ` ` = vide)

L'ordre du if/continue garantit qu'un robot sur une ressource masque la ressource, et que le joueur masque tout.

### `draw_stats`

Affiche en deux lignes :
1. Compteurs de jeu (énergie, cristaux, ressources restantes/connues, robots)
2. Raccourcis clavier avec état actuel (coût cassage mur, état ralliement ON/OFF)

---

## `draw_victory_dialog`

Popup centrée (76×18 caractères) rendue **par-dessus** la simulation (appelée après `draw_simulation`). Utilise `f.render_widget(Clear, popup_area)` pour effacer la zone avant de dessiner le dialog — sinon la carte en dessous transparaîtrait.

Affiche le temps écoulé (formaté `Xm Ys`), les ressources gagnées cette partie, le total accumulé, et l'état des favoris (`[*]` si favorisée, `[ ]` sinon).

---

## Justifications de design

- **Pas d'état dans `ui.rs`** : toutes les fonctions prennent leurs données en paramètre et ne stockent rien. Le rendu est une pure fonction de l'état → pas de désynchronisation possible.
- **Rendu complet à chaque tick** : ratatui fonctionne en "full redraw" — on redessine tout le frame à chaque itération plutôt que de gérer des mises à jour partielles. Avec une carte 120×40 = 4 800 caractères, c'est largement dans les capacités d'un terminal moderne (~80 ms/tick).
