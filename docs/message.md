# `src/message.rs` — Protocole de communication robots → base

## Rôle du fichier

Définit les trois événements que les threads robots peuvent envoyer au thread `base_processor` via le canal `mpsc`. C'est le seul moyen qu'ont les robots de modifier les compteurs de ressources partagés.

---

## `Message`

```rust
pub enum Message {
    ResourceDiscovered { pos, kind, quantity },
    ResourceCollected  { pos, kind, amount   },
    ResourceDepleted   { pos                 },
}
```

### `ResourceDiscovered`
Émis par un **scout** quand il voit un gisement dans son rayon de vision (`VISION_RANGE = 4`) qui n'est pas encore dans `BaseKnowledge`. Le `base_processor` l'enregistre dans `knowledge.resources` (avec `or_insert` — si deux scouts découvrent la même case en même temps, le premier l'enregistre, le second est ignoré sans erreur).

### `ResourceCollected`
Émis par un **collecteur** quand il revient à la base et décharge sa cargaison. Le `base_processor` :
1. Incrémente `collected_energy` / `collected_crystals` et `available_energy`.
2. Décrémente la quantité connue dans `knowledge.resources` (ou supprime l'entrée si épuisée).

### `ResourceDepleted`
Émis par un **collecteur** quand il constate que le gisement qu'il était en train de récolter atteint 0. Permet à `base_processor` de retirer immédiatement l'entrée de `knowledge.resources` pour que les autres collecteurs ne cherchent plus à y aller.

---

## Pourquoi un canal `mpsc` et non écrire directement dans `SimState` ?

Les collecteurs écrivent déjà dans `SimState` (mise à jour de `resources`, `reserved`, position des robots) pendant leur tick — sous verrou `Mutex`. Si on y mettait aussi les mises à jour de compteurs (`collected_energy`...), on risquerait le double-comptage : deux threads pourraient traiter le même retour à la base en interleaving.

Avec le canal `mpsc` + `base_processor`, **un seul thread** traite ces événements séquentiellement. C'est un pattern producteur/consommateur classique qui garantit la cohérence sans verrou supplémentaire sur les compteurs.

---

## Durée de vie du canal

Dans `start_simulation()` :
```rust
let (msg_tx, msg_rx) = mpsc::channel::<Message>();
// clones de msg_tx distribués à chaque robot
drop(msg_tx);  // le sender "maître" est dropped ici
// base_processor reçoit msg_rx et boucle jusqu'à ce que tous les Senders soient dropped
```

Quand tous les threads robots s'arrêtent (drapeau `running = false`), ils sortent de leur boucle et droppent leurs clones de `tx`. Le channel se ferme naturellement, `base_processor` sort de `for msg in rx { ... }` et se termine — sans avoir besoin de tester lui-même le drapeau `running`.
