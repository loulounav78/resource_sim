# `src/robot.rs` — Logique des threads robots

## Rôle du fichier

Contient les trois fonctions qui tournent chacune dans leur propre thread OS : `run_scout`, `run_collector`, et `base_processor`. C'est là que vit toute la logique comportementale autonome des robots.

---

## Architecture thread

```
thread::spawn → run_scout(0)       ┐
thread::spawn → run_scout(1)       │  N scouts
thread::spawn → run_scout(...)     ┘
thread::spawn → run_collector(N)   ┐
thread::spawn → run_collector(N+1) │  M collectors
thread::spawn → run_collector(...) ┘
thread::spawn → base_processor     ←  1 thread de réception
```

Chaque robot a un `robot_idx` fixe attribué à la création. Il ne change jamais — c'est l'index dans `SimState::robots[]` que ce thread a le droit de modifier.

---

## `run_scout` — exploration

**Tick** : `SCOUT_TICK_MS = 160 ms`

Boucle :
1. **Vision** : parcourt une zone `VISION_RANGE×VISION_RANGE = 4` cases autour de sa position. Pour chaque ressource visible non encore connue de la base → envoie `Message::ResourceDiscovered`.
2. **Déplacement** :
   - Si **ralliement actif** (`scout_rally_pos`) → BFS vers le point de ralliement, esquive si bloqué.
   - Sinon → **marche biaisée par compteur de visites** : parmi les voisins libres, choisit celui(ceux) le(s) moins visité(s), tirage aléatoire en cas d'égalité.

### Pourquoi le biais de visite et pas une marche purement aléatoire ?

Une marche aléatoire pure a tendance à rester trop longtemps dans les zones déjà explorées (le problème du "drunkard's walk"). Le compteur de visites local (`HashMap<Pos, u32>` privé à ce scout) force une préférence pour les cases fraîches — exploration bien plus efficace en pratique — sans avoir besoin de connaître la carte entière.

---

## `run_collector` — collecte et retour

**Tick** : `COLLECTOR_TICK_MS = 120 ms`

Machine à états locale (variables `returning`, `at_source`) :

```
SEEK ──(ressource trouvée)──► AT_SOURCE ──(plein ou épuisé)──► RETURNING ──(base)──► SEEK
```

### Phase SEEK
1. Si le collecteur est sur une case avec ressource → passe en `AT_SOURCE`.
2. Sinon, si une cible est déjà choisie → avance sur le chemin BFS vers elle (esquive si bloqué).
3. Sinon → appelle `best_target()` pour choisir la ressource connue la plus proche non réservée, la réserve dans `SimState::reserved`, calcule le chemin BFS.
4. Si aucune cible connue → **wandering** : déplacement aléatoire parmi les cases libres.

### Phase AT_SOURCE
Décrémente `resources[pos].quantity` de 1 par tick, incrémente `carry_amount`. Sort quand :
- Le gisement est épuisé → envoie `ResourceDepleted`.
- `carry_amount >= carry_capacity` → cargo plein.

### Phase RETURNING
Suit le chemin BFS vers la base. À l'arrivée → envoie `ResourceCollected` et réinitialise.

---

## `base_processor` — traitement des messages

Boucle `for msg in rx` (bloquant, sans sleep). Traite séquentiellement les messages envoyés par les robots :
- `ResourceDiscovered` → `knowledge.resources.entry(pos).or_insert(...)` (pas d'écrasement si déjà connu)
- `ResourceCollected` → incrémente compteurs, décrémente quantité connue
- `ResourceDepleted` → retire de `knowledge.resources`

Se termine naturellement quand tous les senders (`tx`) sont dropped (les robots se sont arrêtés). C'est pour ça que le `msg_tx` "maître" est dropped explicitement dans `main.rs` avant de spawner ce thread.

---

## `step_aside` — gestion des collisions

Quand la prochaine case du chemin est occupée par un autre robot ou le joueur :

1. Calcule tous les voisins passables de la position actuelle, **sauf** la case bloquée et les cases occupées.
2. Parmi ces alternatives, choisit celle(s) qui minimisent la **distance de Manhattan** vers le but.
3. Tirage aléatoire en cas d'égalité.
4. **`path.clear()`** → le chemin BFS est abandonné.

Le tick suivant, `path.is_empty()` sera vrai → nouveau BFS complet depuis la position post-esquive.

**Trade-off** : simple et correct (jamais de blocage permanent, jamais de chemin périmé suivi) mais coûteux — un BFS complet (jusqu'à 4 800 cases) est relancé après chaque esquive, **pendant que le `Mutex<SimState>` est tenu**.

---

## `occupied_by_others` + `best_target`

- `occupied_by_others(s, self_idx)` : collecte les positions de tous les autres robots + le joueur → `HashSet<Pos>`. Snapshot cohérent car pris sous verrou.
- `best_target(s, pos, priority)` : cherche dans `knowledge.resources` la ressource accessible (présente dans `resources`, non réservée) la plus proche selon la priorité (`Energy` first / `Crystal` first / `None` = la plus proche).
