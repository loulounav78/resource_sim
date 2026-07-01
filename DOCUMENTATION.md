# Documentation — Resource Simulation

Ce document explique comment le projet est construit, comment les différentes
parties du code communiquent entre elles, et définit les termes techniques
employés (Rust, concurrence, génération procédurale...) pour quelqu'un qui
découvre le code.

---

## 1. C'est quoi ce projet ?

Un jeu en **TUI** (interface en mode texte, dans le terminal) écrit en Rust.

Le joueur choisit un niveau (ou relance une carte favorite / une seed
précise), une carte est générée procéduralement (obstacles + dépôts
d'énergie/cristaux), puis des **robots autonomes** (scouts + collecteurs)
explorent la carte et ramènent les ressources à la base tout seuls, pendant
que le joueur contrôle un "vaisseau" qui peut se déplacer et casser des murs
pour aider. La partie est gagnée quand toutes les ressources ont été
récoltées. Les ressources gagnées servent à acheter des améliorations dans
un **store** (plus de robots, plus de cargo, etc.).

---

## 2. Glossaire des termes techniques

### Rust / langage

- **`enum`** : un type qui ne peut prendre qu'une valeur parmi plusieurs
  variantes possibles (parfois avec des données attachées à chaque variante).
  Le projet l'utilise énormément comme **machine à états** (voir `Screen`
  plus bas) : on sait exactement dans quel "écran" on se trouve, et le
  compilateur force à gérer tous les cas.
- **`match`** : équivalent d'un `switch` qui force à traiter (ou à ignorer
  explicitement via `_`) tous les cas possibles d'un `enum`.
- **`struct`** : un type qui regroupe des champs nommés (équivalent d'une
  classe sans méthodes virtuelles).
- **`impl`** : bloc où l'on définit les méthodes associées à une `struct`/`enum`.
- **`Option<T>`** : soit `Some(valeur)`, soit `None`. Remplace le `null` —
  le compilateur oblige à vérifier les deux cas, donc pas de
  "null pointer exception" possible.
- **`Result<T, E>`** : soit `Ok(valeur)`, soit `Err(erreur)`. Utilisé pour les
  opérations qui peuvent échouer (ex: parser une seed tapée par l'utilisateur).
- **Ownership / borrowing (`&`, `&mut`)** : le système qui garantit à la
  compilation qu'il n'y a jamais deux accès en écriture simultanés (ou un accès
  en écriture en même temps qu'une lecture) à la même donnée, sans recourir à
  un ramasse-miettes (garbage collector). `&x` = emprunt en lecture, `&mut x`
  = emprunt en écriture exclusif.
- **Trait** : l'équivalent d'une interface (ex: `Serialize`, `Deserialize`,
  `Clone`, `Copy`...). `#[derive(...)]` génère automatiquement
  l'implémentation d'un trait simple pour une struct/enum.
- **`Vec<T>`** : tableau dynamique (liste qui peut grandir).
- **`HashMap<K, V>` / `HashSet<T>`** : table de hachage (association
  clé→valeur / ensemble), utilisée ici massivement avec des positions
  `(x, y)` comme clés (`Pos`).

### Concurrence (multi-threading)

- **`thread::spawn`** : démarre un nouveau fil d'exécution qui tourne en
  parallèle du reste du programme. Chaque robot (scout/collecteur) tourne
  dans son propre thread (`src/robot.rs`).
- **`Arc<T>`** (*Atomically Reference Counted*) : pointeur partagé entre
  threads. Permet à plusieurs threads de posséder une référence vers la même
  donnée en mémoire sans copier les données.
- **`Mutex<T>`** (*mutual exclusion*) : verrou qui garantit qu'un seul thread
  à la fois peut lire/modifier la donnée qu'il protège. `state.lock().unwrap()`
  prend le verrou ; il est relâché automatiquement à la fin du bloc.
  `Arc<Mutex<SimState>>` = "l'état de la simulation, partagé et protégé
  entre tous les threads (robots + thread principal)".
- **`AtomicBool`** : booléen qu'on peut lire/écrire depuis plusieurs threads
  sans `Mutex` (ex: le drapeau `running` qui dit à tous les robots de
  s'arrêter quand la partie se termine).
- **`mpsc::channel`** (*multi-producer, single-consumer*) : une file
  d'attente thread-safe. Chaque robot envoie des `Message` (ressource
  trouvée/collectée/épuisée) dans le channel ; un thread unique
  (`base_processor`) les consomme un par un et met à jour l'état partagé.
  Ça évite que tous les robots se battent pour le verrou du `Mutex` en même
  temps pour des mises à jour simples.

### Génération procédurale / RNG

- **Bruit de Perlin (`Perlin noise`)** : algorithme qui génère un champ de
  valeurs continues "naturelles" (pas du bruit aléatoire pur — des zones
  lisses qui varient progressivement, comme un relief). Utilisé ici pour
  décider quelles cases sont des obstacles : si la valeur de bruit à une
  position dépasse un seuil (`0.22`), la case devient un mur.
- **Seed** : nombre de départ qui détermine entièrement le résultat d'un
  générateur pseudo-aléatoire. La même seed → toujours la même carte.
  C'est ce qui permet aux "cartes favorites" d'être rejouées à l'identique.
- **RNG** (*Random Number Generator*) : générateur de nombres
  pseudo-aléatoires. `rand::random()` tire un nombre vraiment imprévisible
  (basé sur l'horloge/le système) ; `StdRng::seed_from_u64(seed)` crée un
  générateur **déterministe** à partir d'une seed donnée.
- **Mixage de seed (SplitMix64)** : la crate `noise` ne "disperse" pas bien
  les petites valeurs de seed (1, 2, 3...) dans sa table de permutation
  interne, ce qui produisait des terrains presque identiques pour des seeds
  proches. `mix_seed()` (dans `src/map.rs`) applique une fonction de hachage
  reconnue (SplitMix64) à la seed avant de l'utiliser, pour que même une
  seed à un seul chiffre tapée à la main donne une carte bien distincte.

### Interface texte (TUI)

- **TUI** (*Text User Interface*) : une interface graphique mais dessinée
  avec des caractères dans un terminal, plutôt qu'avec des pixels.
- **Ratatui** : la bibliothèque Rust utilisée pour dessiner l'interface
  (`src/ui.rs`). On y manipule des `Block` (boîtes avec bordures), des
  `Paragraph` (texte), des `Layout` (découpage de l'écran en zones
  horizontales/verticales avec des `Constraint` de taille).
- **Crossterm** : la bibliothèque bas niveau qui gère le terminal lui-même
  (mode brut/"raw mode" pour lire les touches sans attendre Entrée, capture
  souris, écran alternatif...).
- **`Frame`** : la "toile" sur laquelle on dessine à chaque tour de boucle
  (`terminal.draw(|f| ...)`).

### Persistance / sérialisation

- **`serde` / `serde_json`** : bibliothèques qui transforment automatiquement
  une `struct` Rust en JSON et inversement (`#[derive(Serialize,
  Deserialize)]`). Utilisées pour la sauvegarde (`save.json`).
- **`#[serde(default)]`** : si un champ n'existe pas dans un ancien
  `save.json` (ex: avant l'ajout des favoris), il est rempli avec une valeur
  par défaut au chargement plutôt que de faire planter le parsing —
  important pour la compatibilité avec les anciennes sauvegardes.

### Algorithmes

- **BFS** (*Breadth-First Search*, parcours en largeur) : algorithme de
  recherche de chemin (`src/pathfinding.rs`) qui explore la carte case par
  case en largeur depuis le départ jusqu'à trouver l'arrivée. Garantit le
  chemin le plus court en nombre de cases (pas de notion de "distance à vol
  d'oiseau" ici, donc BFS suffit — pas besoin d'A*).
- **Distance de Manhattan** : `|x1-x2| + |y1-y2|`. Utilisée comme estimation
  rapide de distance (sans tenir compte des murs) pour départager des choix.

---

## 3. Organisation des fichiers (modules)

```
src/
├── main.rs        Boucle de jeu, machine à états des écrans, gestion des touches
├── ui.rs           Tout le rendu visuel (Ratatui)
├── map.rs          Génération de la carte (bruit de Perlin + seed)
├── simulation.rs   Etat de la simulation en cours (SimState) + actions du joueur
├── robot.rs        Logique des threads scouts/collecteurs + traitement des messages
├── pathfinding.rs  BFS + recherche de voisins praticables
├── message.rs      Les messages envoyés par les robots vers le thread principal
├── types.rs        Types partagés simples (Pos, Direction, Resource...)
├── config.rs       Configuration d'une partie (nombre de robots, etc.)
├── levels.rs       Définition statique des 5 niveaux de difficulté
├── store.rs        Catalogue d'améliorations et logique d'achat
└── save.rs         Sauvegarde/chargement JSON (save.json), favoris
```

Chaque fichier est un **module** Rust déclaré dans `main.rs` via `mod xxx;`.

---

## 4. La boucle de jeu et la machine à états (`main.rs`)

Le cœur de `main.rs` est une boucle infinie (`'main: loop { ... }`) qui fait,
à chaque itération :

1. **Render** : dessine l'écran courant.
2. **Détection de victoire** : si on est en partie et qu'il ne reste plus
   aucune ressource sur la carte, bascule vers l'écran de victoire.
3. **Lecture des entrées clavier** (avec un timeout, voir plus bas) et
   exécution de l'action correspondante.

L'écran courant est représenté par l'enum `Screen` :

```rust
enum Screen {
    MainMenu { selected, focus, fav_selected, seed_input },
    Store { selected_item, feedback },
    Running { state, handle, start_time, rally_until, rally_shift_held, level, seed },
    VictoryDialog { state, elapsed_secs, energy_earned, crystals_earned, level, seed },
}
```

C'est une **machine à états** classique : à tout instant, le programme est
dans exactement un de ces quatre états, et chaque variante transporte
les données dont cet écran précis a besoin (ex: `Running` transporte l'état
partagé de la simulation et la seed utilisée pour pouvoir la mettre en
favori).

Les transitions entre écrans passent par un enum interne `Action`
(`StartLevel`, `OpenStore`, `ToggleFavorite`, etc.) : on calcule d'abord
*quelle action* la touche pressée déclenche, puis on l'exécute dans un second
`match`. Ça sépare clairement "qu'est-ce que l'utilisateur veut faire" de
"qu'est-ce que ça change dans l'état du jeu".

### Le menu principal et ses 3 zones de focus

Le menu principal (`MainMenu`) a un système de **focus** (`MainMenuFocus`)
à 3 valeurs : `Levels` (cartes de niveau), `SeedInput` (champ de saisie de
seed) et `Favorites` (liste des cartes favorites). Les flèches Haut/Bas
déplacent le focus entre ces 3 zones ; Gauche/Droite ne s'appliquent qu'à
l'intérieur de la zone `Levels`. Quand le focus est sur `SeedInput`, les
touches numériques sont interceptées pour écrire dans le champ texte au lieu
de déclencher les raccourcis habituels (ex: `S` n'ouvre pas le store pendant
la saisie).

### Pourquoi un timeout sur la lecture clavier ?

```rust
let timeout = tick.saturating_sub(last_tick.elapsed());
if event::poll(timeout)? { ... }
```

`UI_TICK_MS = 80` : le programme ne reste jamais bloqué plus de 80 ms à
attendre une touche. Ça permet à l'écran de se rafraîchir régulièrement même
si l'utilisateur n'appuie sur rien (utile pendant `Running`, où la
simulation évolue toute seule en arrière-plan grâce aux threads des robots).

---

## 5. Génération de la carte (`map.rs`)

`Map::generate(width, height, seed, energy_range, crystal_range)` :

1. **Terrain** : pour chaque case, calcule une valeur de bruit de Perlin
   normalisée sur une grille `6.0 × 6.0` (le facteur `6.0` contrôle la
   "fréquence" du bruit, donc la taille des blocs d'obstacles). Si la valeur
   dépasse `0.22`, la case devient un obstacle avec une durabilité de
   `WALL_MAX_DURABILITY` (3 — il faut 3 points de dégâts pour le détruire).
2. **Base** : une zone 5×5 au centre de la carte est nettoyée et marquée
   `Tile::Base`.
3. **Ressources** : une RNG **dérivée de la même seed** (`StdRng`) place des
   dépôts d'énergie/cristal aléatoirement sur les cases vides, jusqu'à
   atteindre un quota (`largeur × hauteur / 35`).

La seed brute est d'abord passée dans `mix_seed()` (voir glossaire) avant
d'être utilisée, pour garantir que deux seeds proches donnent des cartes
visuellement très différentes.

---

## 6. La simulation en jeu (`simulation.rs`, `robot.rs`, `message.rs`)

### État partagé : `SimState`

`SimState` contient tout ce qui doit être visible/modifiable à la fois par
le thread principal (rendu + joueur) et par les threads des robots : la
carte, les ressources restantes, la position des robots, la position du
joueur, ce que la base "sait" déjà (`BaseKnowledge`), les compteurs de
ressources collectées, etc. Il vit dans un `Arc<Mutex<SimState>>` partagé
par tous les threads.

### Les threads

`start_simulation()` (dans `main.rs`) lance :

- **N threads `run_scout`** : chaque scout explore la carte (déplacement
  aléatoire pondéré pour éviter de repasser sans cesse au même endroit) et,
  dans son rayon de vision (`VISION_RANGE = 4`), détecte les ressources
  qu'il ne connaît pas encore et envoie un `Message::ResourceDiscovered`.
  S'il y a un point de ralliement actif (le joueur a appuyé sur Shift), il
  s'y dirige plutôt que d'explorer.
- **N threads `run_collector`** : chaque collecteur va chercher la ressource
  connue la plus proche (et pas déjà réservée par un autre collecteur — voir
  `reserved` dans `SimState`), la collecte unité par unité jusqu'à atteindre
  sa capacité de cargo ou l'épuisement du gisement, puis revient à la base
  pour décharger (`Message::ResourceCollected`).
- **1 thread `base_processor`** : consomme les messages envoyés par tous les
  robots via le channel `mpsc` et met à jour `SimState` en conséquence
  (connaissance de la base, compteurs d'énergie/cristaux). C'est le seul
  endroit qui traite ces événements, donc pas de risque de double-comptage.

Tous ces threads tournent en boucle tant que le drapeau partagé `running`
(un `AtomicBool`) est à `true`. Quand le joueur quitte la partie,
`SimHandle::stop()` met `running` à `false` et attend (`.join()`) que tous
les threads se terminent proprement avant de continuer.

### Le joueur

Le joueur ne tourne pas dans un thread séparé : ses actions (`move_player`,
`damage_wall_from_player`) sont appelées directement depuis la boucle
principale en réaction aux touches ZQSD / Ctrl+ZQSD, avec verrouillage du
même `Mutex<SimState>` que les robots.

---

## 7. Sauvegarde et favoris (`save.rs`)

`SaveData` est sérialisée en JSON dans `save.json` à chaque événement
important (victoire, achat au store, fin de partie). Elle contient la
progression du joueur (ressources totales, nombre de robots, bonus
d'amélioration...) **et**, désormais, `favorite_maps: Vec<FavoriteMap>` —
chaque `FavoriteMap` est juste une paire `(seed, level)`.

- `is_favorite(seed)` : vérifie si une seed est dans la liste.
- `toggle_favorite(level, seed)` : ajoute la carte si elle n'y est pas,
  la retire sinon (bouton "bascule").

Comme la génération de carte est entièrement déterministe à partir de la
seed (terrain **et** ressources, voir section 5), relancer une carte
favorite avec `Action::StartFavorite` reproduit exactement la même partie.

---

## 8. Store et niveaux (`store.rs`, `levels.rs`)

- `levels.rs` définit statiquement 5 niveaux de difficulté (`LEVELS`), chacun
  avec une plage min/max de quantité d'énergie/cristaux par dépôt.
- `store.rs` définit un catalogue fixe d'améliorations (`STORE_ITEMS`) :
  plus de scouts, plus de collecteurs, plus de cargo, meilleurs dépôts,
  meilleur cassage de mur. Le coût d'un item augmente à chaque achat
  (`item_cost` multiplie le coût de base par le nombre d'achats déjà faits,
  sauf pour le cassage de mur qui a un coût fixe par palier).

---

## 9. L'interface (`ui.rs`)

Chaque écran a sa fonction de dessin (`draw_main_menu`, `draw_store`,
`draw_simulation`, `draw_victory_dialog`). Le pattern général :

1. Découper la zone disponible (`f.area()`) en sous-zones avec un `Layout`
   (`Constraint::Length(n)` = taille fixe en lignes/colonnes,
   `Constraint::Min(n)` = prend le reste de l'espace disponible).
2. Pour chaque sous-zone, dessiner un `Block` (bordure + titre) puis un
   `Paragraph` (texte stylé avec `Span`/`Line`) à l'intérieur.
3. La couleur/le style change selon ce qui est sélectionné/focus (ex: bordure
   jaune = élément actif).

La carte elle-même (`draw_map`) est dessinée caractère par caractère : `V`
pour le vaisseau du joueur, `x`/`o`/`@` pour les robots, `E`/`C` pour les
ressources, `O`/`Ø`/`¤` pour les murs selon leur durabilité restante.

---

## 10. Contrôles en jeu (résumé)

| Touche | Effet |
|---|---|
| ← / → | Choisir un niveau (menu) |
| ↑ / ↓ | Changer de zone de focus (menu) |
| Entrée | Lancer / valider |
| Suppr | Retirer la carte sélectionnée des favoris (menu) |
| S | Ouvrir le store (menu) |
| Z Q S D | Déplacer le vaisseau (en jeu) |
| Ctrl + Z Q S D | Casser un mur dans cette direction (coûte de l'énergie) |
| Maj (Shift) | Rallier les scouts vers le vaisseau (coûte de l'énergie) |
| F | Ajouter/retirer la carte actuelle des favoris (en jeu / écran de victoire) |
| Échap | Retour / quitter |

---

## 11. Pour lancer le projet

```bash
cargo run
```

`cargo check` permet de vérifier que le code compile sans générer
l'exécutable final (utile si l'exécutable précédent est encore en cours
d'exécution et verrouillé par Windows).
