# 06 — Battle subsystem and dev console

Covers **M7** (battle subsystem) and **M8** (dev console FFI).

## Battle (M7)

`adventerm_ffi/src/battle.rs`. Handle defined in `handle.rs` per [01](01-crate-and-handles.md):

```rust
#[repr(C)]
pub struct BattleHandle { pub(crate) inner: adventerm_lib::Battle }
```

**Battle is an independent owner.** `battle_start` clones the snapshot it needs from `GameState` (the lib's `start_battle` already produces a self-contained `Battle`). `BattleHandle` does NOT borrow from `GameHandle`. The two can be freed in any order.

### Lifecycle

```rust
#[no_mangle]
pub extern "C" fn battle_start(
    game: *const GameHandle,
    enemy_entity: u32,
    out_battle: *mut *mut BattleHandle,
    out_started: *mut bool,           // false if start_battle returned None
) -> i32;

#[no_mangle]
pub extern "C" fn battle_free(battle: *mut BattleHandle);
```

### Turn dispatch

```rust
#[no_mangle]
pub extern "C" fn battle_apply_player_ability(
    game: *const GameHandle,
    battle: *mut BattleHandle,
    ability_slot: usize,
) -> i32;  // 0 = Ok; -4 = EmptySlot; -5 = NotPlayerTurn; -6 = AlreadyResolved

#[no_mangle]
pub extern "C" fn battle_apply_enemy_turn(
    game: *const GameHandle,
    battle: *mut BattleHandle,
) -> i32;
```

Note both take `*const GameHandle` (not `*mut`) — the lib's engine functions take `&GameState`, not `&mut`. Battle state mutates only the `Battle` itself; player HP changes via `game_set_cur_health` after the battle resolves.

### State queries

```rust
#[no_mangle]
pub extern "C" fn battle_turn(
    battle: *const BattleHandle,
    out_turn: *mut CBattleTurn,        // tagged: 0=Player, 1=Enemy, 2=Resolved + result byte
) -> i32;

#[no_mangle]
pub extern "C" fn battle_combatants(
    battle: *const BattleHandle,
    out_combatants: *mut CCombatants,
) -> i32;

#[no_mangle]
pub extern "C" fn battle_player_cur_hp(
    battle: *const BattleHandle,
    out_hp: *mut u8,
) -> i32;

#[no_mangle]
pub extern "C" fn battle_enemy_cur_hp(
    battle: *const BattleHandle,
    out_hp: *mut u8,
) -> i32;

#[no_mangle]
pub extern "C" fn battle_hp_snapshot(
    battle: *const BattleHandle,
    out_hp: *mut CHpSnapshot,
) -> i32;

#[no_mangle]
pub extern "C" fn battle_is_resolved(
    battle: *const BattleHandle,
    out: *mut bool,
) -> i32;

#[no_mangle]
pub extern "C" fn battle_result(
    battle: *const BattleHandle,
    out_result: *mut u8,               // CBattleResult, valid iff out_has_result is true
    out_has_result: *mut bool,
) -> i32;
```

### Log access (Pattern C from [05](05-iteration-and-save.md))

```rust
#[no_mangle]
pub extern "C" fn battle_log_line_count(
    battle: *const BattleHandle,
    out_count: *mut usize,
) -> i32;

#[no_mangle]
pub extern "C" fn battle_log_line_copy(
    battle: *const BattleHandle,
    index: usize,
    buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32;
```

### Test plan (M7)

`adventerm_ffi/tests/battle_flow.rs`:

```rust
#[test]
fn scripted_battle_resolves_to_known_outcome() {
    // Use a seed that places an enemy adjacent to the player
    let h = adventerm_ffi::game_new_seeded(KNOWN_BATTLE_SEED);

    // Walk into the enemy to set pending encounter
    // ... game_action_move chain ...

    // Pull the encounter
    let mut entity = 0u32;
    let mut has = false;
    adventerm_ffi::game_take_pending_encounter(h, &mut entity, &mut has);
    assert!(has);

    // Start battle
    let mut battle: *mut adventerm_ffi::BattleHandle = std::ptr::null_mut();
    let mut started = false;
    adventerm_ffi::battle_start(h, entity, &mut battle, &mut started);
    assert!(started);

    // Apply abilities until resolved
    for _ in 0..50 {
        let mut turn = adventerm_ffi::CBattleTurn::default();
        adventerm_ffi::battle_turn(battle, &mut turn);
        match turn.tag {
            0 => { adventerm_ffi::battle_apply_player_ability(h, battle, 0); }
            1 => { adventerm_ffi::battle_apply_enemy_turn(h, battle); }
            2 => break,
            _ => panic!("unknown turn tag"),
        }
    }

    let mut resolved = false;
    adventerm_ffi::battle_is_resolved(battle, &mut resolved);
    assert!(resolved);

    // Read log
    let mut count = 0usize;
    adventerm_ffi::battle_log_line_count(battle, &mut count);
    assert!(count > 0);

    adventerm_ffi::battle_free(battle);
    adventerm_ffi::game_free(h);
}

#[test]
fn battle_start_returns_false_for_no_enemy() {
    let h = adventerm_ffi::game_new_seeded(42);
    let mut battle: *mut adventerm_ffi::BattleHandle = std::ptr::null_mut();
    let mut started = true;
    adventerm_ffi::battle_start(h, /* invalid entity */ 0, &mut battle, &mut started);
    assert!(!started);
    assert!(battle.is_null());
    adventerm_ffi::game_free(h);
}

#[test]
fn battle_apply_player_ability_wrong_turn_returns_not_player_turn() {
    // Set up a battle, apply enemy turn first via direct manipulation, then try player ability
}
```

## Dev console (M8)

`adventerm_ffi/src/console.rs`. Handle:

```rust
#[repr(C)]
pub struct ConsoleHandle { pub(crate) inner: adventerm_lib::console::ConsoleState }
```

**Console operates on a borrowed `&mut GameState` per call** — the FFI surface preserves this by taking `*mut GameHandle` alongside the `*mut ConsoleHandle` for any operation that might mutate game state.

### Lifecycle

```rust
#[no_mangle]
pub extern "C" fn console_new(out_console: *mut *mut ConsoleHandle) -> i32;

#[no_mangle]
pub extern "C" fn console_free(console: *mut ConsoleHandle);
```

### Editing

```rust
#[no_mangle]
pub extern "C" fn console_input_get(
    console: *const ConsoleHandle,
    buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32;

#[no_mangle]
pub extern "C" fn console_input_set(
    console: *mut ConsoleHandle,
    text: *const c_char,
) -> i32;

#[no_mangle]
pub extern "C" fn console_cursor(
    console: *const ConsoleHandle,
    out_pos: *mut usize,
) -> i32;

#[no_mangle]
pub extern "C" fn console_set_cursor(
    console: *mut ConsoleHandle,
    pos: usize,
) -> i32;

#[no_mangle]
pub extern "C" fn console_insert_char(
    console: *mut ConsoleHandle,
    codepoint: u32,                  // Rust char as Unicode scalar
) -> i32;

#[no_mangle]
pub extern "C" fn console_delete_back(console: *mut ConsoleHandle) -> i32;

#[no_mangle]
pub extern "C" fn console_clear(console: *mut ConsoleHandle) -> i32;
```

### Submission

```rust
#[no_mangle]
pub extern "C" fn console_submit(
    console: *mut ConsoleHandle,
    game: *mut GameHandle,
    out_response: *mut u8,           // copies command output (or empty)
    cap: usize,
    out_required: *mut usize,
    out_was_error: *mut bool,        // true if command returned Err(_)
) -> i32;
```

`console_submit` runs the command parser, dispatches the matched `DevCommand` with `DevCtx { game: Some(&mut game.inner) }`, and copies the resulting string into the caller's buffer. The trait object stays internal.

### History (Pattern C)

```rust
#[no_mangle]
pub extern "C" fn console_history_count(
    console: *const ConsoleHandle,
    out_count: *mut usize,
) -> i32;

#[no_mangle]
pub extern "C" fn console_history_line_copy(
    console: *const ConsoleHandle,
    index: usize,
    buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32;
```

### Completions

```rust
#[no_mangle]
pub extern "C" fn console_complete(
    console: *mut ConsoleHandle,
    game: *const GameHandle,         // for CompletionCtx
) -> i32;

#[no_mangle]
pub extern "C" fn console_completion_count(
    console: *const ConsoleHandle,
    out_count: *mut usize,
) -> i32;

#[no_mangle]
pub extern "C" fn console_completion_at(
    console: *const ConsoleHandle,
    index: usize,
    buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32;
```

### Static command introspection

The `DevCommand` registry is a `&'static [&'static dyn DevCommand]` — trait objects don't cross. Expose by-name access:

```rust
#[no_mangle]
pub extern "C" fn console_command_count(out_count: *mut usize) -> i32;

#[no_mangle]
pub extern "C" fn console_command_name(
    index: usize,
    buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32;

#[no_mangle]
pub extern "C" fn console_command_help(
    index: usize,
    buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32;
```

The trait method `arg_completions` is exercised through `console_complete`, which runs internally and cooks the candidate strings — the trait object never leaves Rust.

### Test plan (M8)

`adventerm_ffi/tests/console_flow.rs`:

```rust
#[test]
fn submit_fullbright_flips_game_state() {
    let h = adventerm_ffi::game_new_seeded(42);
    let mut console: *mut adventerm_ffi::ConsoleHandle = std::ptr::null_mut();
    adventerm_ffi::console_new(&mut console);

    let cmd = std::ffi::CString::new("fullbright").unwrap();
    adventerm_ffi::console_input_set(console, cmd.as_ptr());

    let mut response = vec![0u8; 256];
    let mut needed = 0usize;
    let mut was_error = false;
    adventerm_ffi::console_submit(
        console, h,
        response.as_mut_ptr(), response.len(),
        &mut needed, &mut was_error,
    );
    assert!(!was_error);

    let mut on = false;
    adventerm_ffi::game_fullbright(h, &mut on);
    assert!(on);

    adventerm_ffi::console_free(console);
    adventerm_ffi::game_free(h);
}

#[test]
fn submit_unknown_command_sets_was_error() {
    // Submit "asdf"; assert was_error = true; response contains "unknown command"
}

#[test]
fn complete_lists_known_commands() {
    let mut console: *mut adventerm_ffi::ConsoleHandle = std::ptr::null_mut();
    adventerm_ffi::console_new(&mut console);

    let cmd = std::ffi::CString::new("ful").unwrap();
    adventerm_ffi::console_input_set(console, cmd.as_ptr());

    adventerm_ffi::console_complete(console, std::ptr::null());

    let mut count = 0usize;
    adventerm_ffi::console_completion_count(console, &mut count);
    assert!(count >= 1);

    adventerm_ffi::console_free(console);
}

#[test]
fn command_registry_introspection() {
    let mut count = 0usize;
    adventerm_ffi::console_command_count(&mut count);
    assert!(count >= 3);  // fullbright, spawn, give

    let mut name_buf = vec![0u8; 64];
    let mut needed = 0usize;
    adventerm_ffi::console_command_name(0, name_buf.as_mut_ptr(), name_buf.len(), &mut needed);
    let name = std::ffi::CStr::from_bytes_with_nul(&name_buf[..needed]).unwrap();
    assert!(matches!(name.to_str().unwrap(), "fullbright" | "spawn" | "give"));
}
```

## Combined notes

- Both Battle and Console handles store **independent** state. They do not borrow from `GameHandle`. Free order doesn't matter.
- The console exists in `adventerm_lib::console`, not the binary. Re-confirmed by searching the source — moving it would break the lib boundary.
- Both subsystems use Pattern C (count + line_copy) for any `Vec<String>`-style output.
