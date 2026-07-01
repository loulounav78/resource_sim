# `src/main.rs` — Boucle principale et machine à états

## Rôle du fichier

Point d'entrée du programme. Gère la boucle de jeu principale, la machine à états des écrans, les événements clavier, et le cycle de vie des threads de simulation.

---

## Constantes

```rust
const MAP_WIDTH: usize  = 120;
const MAP_HEIGHT: usize = 40;
const UI_TICK_MS: u64   = 80;   // ~12.5 FPS
const RALLY_HOLD_GRACE_MS: u64 = 300;
```

`UI_TICK_MS = 80` : le programme ne bloque jamais plus de 80 ms sur l'attente d'une touche. Cela permet de rafraîchir l'affichage même en l'absence d'input utilisateur — nécessaire pour voir les robots se déplacer.

---

## `SimHandle` — cycle de vie de la simulation

```rust
struct SimHandle {
    running: Arc<AtomicBool>,
    handles: Vec<JoinHandle<()>>,
}

impl SimHandle {
    fn stop(self) {
        self.running.store(false, Ordering::Relaxed);
        for h in self.handles { let _ = h.join(); }
    }
}
```

Agrège le drapeau d'arrêt et les handles de tous les threads. `stop()` : met `running` à `false` puis attend que tous les threads se terminent proprement. Les threads vérifient `running` au début de chaque tick (après leur `sleep`), donc le délai d'arrêt est au pire `max(SCOUT_TICK_MS, COLLECTOR_TICK_MS) = 160 ms`.

---

## `build_config` — construction de la config de partie

```rust
fn build_config(level_idx: usize, save: &SaveData) -> Config {
    let lvl = &LEVELS[level_idx];
    Config {
        num_scouts:    save.num_scouts,
        num_collectors: save.num_collectors,
        priority:      Priority::None,
        carry_capacity: save.carry_capacity,
        energy_min:    lvl.energy_min + save.energy_bonus,
        energy_max:    lvl.energy_max + save.energy_bonus,
        crystal_min:   lvl.crystal_min + save.crystal_bonus,
        crystal_max:   lvl.crystal_max + save.crystal_bonus,
        wall_break_power: save.wall_break_power,
    }
}
```

Fusionne progression du joueur (save) + difficulté du niveau (LEVELS) en un objet `Config` immuable passé à `start_simulation`. Les bonus du store s'ajoutent aux plages du niveau.

---

## `start_simulation` — lancement des threads

```rust
fn start_simulation(cfg: &Config, seed: u32) -> (Arc<Mutex<SimState>>, SimHandle)
```

1. Génère la carte (`Map::generate`).
2. Initialise `SimState` avec les robots positionnés à la base.
3. Spawn **un thread par scout** (`run_scout(i, ...)`)
4. Spawn **un thread par collecteur** (`run_collector(num_scouts + i, ...)`)
5. Drop `msg_tx` — le sender "maître" du channel (important : garantit la fermeture automatique du channel quand tous les robots s'arrêtent).
6. Spawn **`base_processor`** — consomme les messages entrants.

Retourne `(state, SimHandle)` où `state` est le `Arc<Mutex<SimState>>` partagé entre tous les threads et le thread principal.

---

## `Screen` — machine à états des écrans

```rust
enum Screen {
    MainMenu   { selected, focus, fav_selected, seed_input },
    Store      { selected_item, feedback },
    Running    { state, handle, start_time, rally_until, rally_shift_held, level, seed },
    VictoryDialog { state, elapsed_secs, energy_earned, crystals_earned, level, seed },
}
```

À tout instant, le programme est exactement dans un des quatre états. Chaque variante transporte uniquement les données dont elle a besoin — `Running` contient le `SimHandle` pour pouvoir arrêter les threads en quittant ; `VictoryDialog` garde le `SimState` pour continuer à afficher la carte en fond.

### Transitions

Les transitions passent par un enum interne `Action` calculé pendant la lecture des touches, puis exécuté dans un second `match`. Ce two-step sépare "qu'est-ce que l'utilisateur veut faire" de "qu'est-ce que ça change dans l'état" — plus lisible et sans borrow double sur `screen`.

---

## Système de ralliement des scouts (Shift)

Trois modes d'activation gérés en parallèle :

| Mode | Mécanisme | Durée |
|---|---|---|
| **Touche Shift maintenue** | `rally_shift_held = true` (détecté via `GetAsyncKeyState` sur Windows ou événements clavier) | Tant que Shift est pressé |
| **Touche Shift relâchée** | `release_scout_rally()` | Fin immédiate (sauf grace) |
| **Grace period** | `rally_until = Some(Instant::now() + 300ms)` | 300 ms après le dernier Shift |

La grace period (`RALLY_HOLD_GRACE_MS = 300`) évite un clignotement visible si l'utilisateur tapote Shift rapidement.

`refresh_scout_rally()` est appelé à chaque début d'itération de la boucle — il met à jour l'état du ralliement dans `SimState` sans attendre un événement clavier.

---

## Détection de victoire

```rust
if state.lock().unwrap().resources.is_empty() {
    // transition vers VictoryDialog
}
```

Vérifiée à chaque tick, **avant** la lecture des événements. Quand `resources` est vide (tous les gisements collectés) :
1. Les ressources gagnées sont ajoutées au total.
2. La sauvegarde est persistée.
3. Les threads sont arrêtés (`handle.stop()`).
4. Transition vers `Screen::VictoryDialog`.

---

## Gestion du terminal

À l'entrée :
```rust
enable_raw_mode()?;
execute!(stdout, EnterAlternateScreen, EnableMouseCapture, PushKeyboardEnhancementFlags(...))?;
```

À la sortie (même en cas d'erreur via `?`) :
```rust
disable_raw_mode()?;
execute!(terminal.backend_mut(), PopKeyboardEnhancementFlags, LeaveAlternateScreen, DisableMouseCapture)?;
terminal.show_cursor()?;
```

`PushKeyboardEnhancementFlags` (avec `REPORT_EVENT_TYPES`) permet de distinguer KeyPress, KeyRepeat et **KeyRelease** — indispensable pour détecter le relâchement de Shift. Sur les terminaux qui ne supportent pas cette extension, l'appel échoue silencieusement (`is_ok()`) et le système de ralliement se rabat sur `GetAsyncKeyState` (Windows) ou le pulse de 300 ms.
