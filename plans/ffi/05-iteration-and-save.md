# 05 — Iteration and save/load

Covers **M5** (iteration accessors) and **M6** (save/load + filesystem helpers).

## Three iteration patterns

We expose collection access via three patterns, picked per-accessor by element shape:

- **Pattern A — copy-element slice** (Copy-element slices like `inventory: &[ItemKind]`): single-call copy-into-buffer.
- **Pattern B — count + index** (heterogeneous compounds like `Enemies::iter_with_pos`): two functions, `_count` and `_at(idx, out_view)`.
- **Pattern C — vec-of-string** (`BattleLog`, console history): `_count` + `_line_copy(idx, buf, cap, out_required)` (string buffer pattern from [02](02-error-and-strings.md)).

**Iterators never cross.** No `*mut Iter<...>` handles. Snapshot semantics at the moment of the count call.

## Pattern A — copy-element slice

For `&[T]` returns where `T: Copy` and the FFI mirror is a single byte (a `Cxxx` enum) or small `#[repr(C)]` struct.

```rust
#[no_mangle]
pub extern "C" fn game_inventory_len(
    handle: *const GameHandle,
    out_len: *mut usize,
) -> i32;

#[no_mangle]
pub extern "C" fn game_inventory_copy(
    handle: *const GameHandle,
    out_buf: *mut u8,           // CItemKind discriminants, one per slot
    cap: usize,
    out_written: *mut usize,
) -> i32;
```

**Implementation pattern:**

```rust
#[no_mangle]
pub extern "C" fn game_inventory_copy(
    handle: *const GameHandle,
    out_buf: *mut u8,
    cap: usize,
    out_written: *mut usize,
) -> i32 {
    crate::ffi_try!({
        let h = match unsafe { handle.as_ref() } {
            Some(h) => h,
            None => return FfiError::NullArgument as i32,
        };
        let inv = h.inner.inventory();
        if !out_written.is_null() {
            unsafe { *out_written = inv.len(); }
        }
        if out_buf.is_null() || cap < inv.len() {
            return FfiError::BufferTooSmall as i32;
        }
        for (i, &kind) in inv.iter().enumerate() {
            unsafe { *out_buf.add(i) = CItemKind::from(kind) as u8; }
        }
        FfiError::Ok as i32
    })
}
```

**Other Pattern A accessors:**

- `game_abilities_active_copy(handle, out_buf: *mut u8, cap=4, out_written) -> i32` — `[Option<AbilityKind>; 4]`, sentinel `255` for empty.
- `game_abilities_passive_copy(handle, ...)` — same pattern, currently always 0 since `PassiveKind` has no variants.
- `game_abilities_learned_active_len/copy` — `Vec<AbilityKind>`.
- `game_abilities_learned_passive_len/copy` — `Vec<PassiveKind>` (always empty for now).
- `game_equipment_snapshot(handle, out: *mut CEquipmentSnapshot) -> i32` — single-call, no count needed (5 fixed slots).

## Pattern B — count + index

For heterogeneous compound iteration where each element is a `#[repr(C)]` struct.

```rust
// Doors in current room
#[no_mangle]
pub extern "C" fn room_doors_count(
    handle: *const GameHandle,
    out_count: *mut usize,
) -> i32;

#[no_mangle]
pub extern "C" fn room_door_at(
    handle: *const GameHandle,
    index: usize,
    out_view: *mut CDoorView,
) -> i32;

// Enemies in a specific room
#[no_mangle]
pub extern "C" fn room_enemies_count(
    handle: *const GameHandle,
    room: u32,
    out_count: *mut usize,
) -> i32;

#[no_mangle]
pub extern "C" fn room_enemy_at(
    handle: *const GameHandle,
    room: u32,
    index: usize,
    out_view: *mut CEnemyView,
) -> i32;

// Light sources in current room
#[no_mangle]
pub extern "C" fn room_lights_count(
    handle: *const GameHandle,
    out_count: *mut usize,
) -> i32;

#[no_mangle]
pub extern "C" fn room_light_at(
    handle: *const GameHandle,
    index: usize,
    out_view: *mut CLightSource,
) -> i32;

// Flares in current room
#[no_mangle]
pub extern "C" fn room_flares_count(
    handle: *const GameHandle,
    out_count: *mut usize,
) -> i32;

#[no_mangle]
pub extern "C" fn room_flare_at(
    handle: *const GameHandle,
    index: usize,
    out_view: *mut CFlareSource,
) -> i32;

// Items at a given tile in current room (rare to have multiple, but safe to iterate)
#[no_mangle]
pub extern "C" fn room_items_at_count(
    handle: *const GameHandle,
    x: usize,
    y: usize,
    out_count: *mut usize,
) -> i32;

#[no_mangle]
pub extern "C" fn room_item_at(
    handle: *const GameHandle,
    x: usize,
    y: usize,
    index: usize,
    out_kind: *mut u8,           // CItemKind
) -> i32;
```

**Lifetime contract** (documented in `agents/ffi.md` and the cbindgen header preamble):

> Indices returned by a `_count` call are valid only until the next FFI mutation on the parent handle (`game_action_*`, `game_set_*`, `game_refresh_visibility`, etc.). Concurrent or interleaved mutation invalidates indices and returns `OutOfRange` if read.

The Swift discipline is: snapshot count, read all indices in a single critical section, then mutate.

## Room introspection helpers

Beyond just iteration, several other read-only room queries:

```rust
#[no_mangle]
pub extern "C" fn game_current_room(handle: *const GameHandle, out_room: *mut u32) -> i32;

#[no_mangle]
pub extern "C" fn game_room_dimensions(
    handle: *const GameHandle,
    room: u32,
    out_width: *mut usize,
    out_height: *mut usize,
) -> i32;

#[no_mangle]
pub extern "C" fn game_room_kind_at(
    handle: *const GameHandle,
    room: u32,
    x: usize,
    y: usize,
    out_kind: *mut CTileKind,
) -> i32;

#[no_mangle]
pub extern "C" fn game_room_walkable(
    handle: *const GameHandle,
    room: u32,
    x: usize,
    y: usize,
    out: *mut bool,
) -> i32;
```

## Save/load surface (M6)

Save/load uses a `SaveHandle` (independent owner of an `adventerm_lib::Save`) and the buffer pattern from [02](02-error-and-strings.md).

### Bytes round-trip

```rust
#[no_mangle]
pub extern "C" fn save_new_from_game(
    handle: *const GameHandle,
    name: *const c_char,
    out_save: *mut *mut SaveHandle,
) -> i32;

#[no_mangle]
pub extern "C" fn save_to_bytes(
    save: *const SaveHandle,
    buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32;

#[no_mangle]
pub extern "C" fn save_from_bytes(
    bytes: *const u8,
    len: usize,
    out_save: *mut *mut SaveHandle,
) -> i32;

#[no_mangle]
pub extern "C" fn save_to_game(
    save: *const SaveHandle,
    out_game: *mut *mut GameHandle,
) -> i32;

#[no_mangle]
pub extern "C" fn save_name(
    save: *const SaveHandle,
    buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32;

#[no_mangle]
pub extern "C" fn save_version(
    save: *const SaveHandle,
    out_version: *mut u32,
) -> i32;

#[no_mangle]
pub extern "C" fn save_free(save: *mut SaveHandle);
```

**Implementation note for `save_to_game`:** clones the inner `GameState` (matches `Save::new` semantics, which moves/clones state). After `save_to_game`, the `SaveHandle` remains valid and independent.

### Filesystem helpers

```rust
#[no_mangle]
pub extern "C" fn save_list_open(
    dir: *const c_char,
    out_listing: *mut *mut SaveListing,
) -> i32;

#[no_mangle]
pub extern "C" fn save_list_count(
    listing: *const SaveListing,
    out_count: *mut usize,
) -> i32;

#[no_mangle]
pub extern "C" fn save_list_name(
    listing: *const SaveListing,
    index: usize,
    buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32;

#[no_mangle]
pub extern "C" fn save_list_path(
    listing: *const SaveListing,
    index: usize,
    buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32;

#[no_mangle]
pub extern "C" fn save_list_modified_unix(
    listing: *const SaveListing,
    index: usize,
    out_unix_seconds: *mut i64,
) -> i32;

#[no_mangle]
pub extern "C" fn save_list_free(listing: *mut SaveListing);

#[no_mangle]
pub extern "C" fn save_delete(path: *const c_char) -> i32;

#[no_mangle]
pub extern "C" fn save_slugify(
    name: *const c_char,
    buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32;

#[no_mangle]
pub extern "C" fn save_slot_path(
    dir: *const c_char,
    name: *const c_char,
    buf: *mut u8,
    cap: usize,
    out_required: *mut usize,
) -> i32;

#[no_mangle]
pub extern "C" fn save_format_version() -> u32;  // Returns SAVE_VERSION constant (currently 9)
```

**Why `SaveListing` is a handle and not just a `count + at` pair returning `(name, path, modified)` directly:**
- `list_saves` walks the directory once and returns a `Vec<SaveSlot>`; the alternative would re-walk on every name/path query.
- Lifetime contract is trivial: listing is read-only after construction. Indices remain valid until `save_list_free`.
- iOS app sandbox passes paths, not URLs; `*const c_char` IN is the right shape.

## Test plan (M5)

`adventerm_ffi/tests/iteration.rs`:

```rust
#[test]
fn inventory_copy_matches_direct() {
    let h = adventerm_ffi::game_new_seeded(42);
    // Pick up an item via dispatch, then verify inventory across both APIs
    // (test fixture would need a seed where pickup is possible)

    let mut len = 0usize;
    adventerm_ffi::game_inventory_len(h, &mut len);
    let mut buf = vec![0u8; len];
    let mut written = 0usize;
    adventerm_ffi::game_inventory_copy(h, buf.as_mut_ptr(), buf.len(), &mut written);
    assert_eq!(written, len);

    // Compare to direct
    // (would need access to the underlying GameState; the FFI test crate has rlib access)

    adventerm_ffi::game_free(h);
}

#[test]
fn inventory_copy_buffer_too_small() {
    // Push items, allocate undersized buffer, expect BufferTooSmall + correct out_written
}

#[test]
fn equipment_snapshot_sentinel_for_empty_slots() {
    // Brand new game has no equipment; all snapshot fields should be u8::MAX (255)
}

#[test]
fn room_door_iteration_matches_direct() {
    // count + at loop; compare against Room::doors() directly
}

#[test]
fn room_enemy_iteration_matches_direct() {
    // count + at loop; compare against Enemies::iter_with_pos() directly
}
```

## Test plan (M6)

`adventerm_ffi/tests/save_round_trip.rs`:

```rust
#[test]
fn bytes_round_trip_preserves_state() {
    let h = adventerm_ffi::game_new_seeded(42);
    // Drive a few actions to dirty the state
    let _ = adventerm_ffi::game_action_move(h, adventerm_ffi::CDirection::Up as u8, &mut Default::default());

    let name = std::ffi::CString::new("test").unwrap();
    let mut save_handle: *mut adventerm_ffi::SaveHandle = std::ptr::null_mut();
    adventerm_ffi::save_new_from_game(h, name.as_ptr(), &mut save_handle);

    let mut needed = 0usize;
    adventerm_ffi::save_to_bytes(save_handle, std::ptr::null_mut(), 0, &mut needed);
    let mut buf = vec![0u8; needed];
    adventerm_ffi::save_to_bytes(save_handle, buf.as_mut_ptr(), buf.len(), &mut needed);

    let mut restored: *mut adventerm_ffi::SaveHandle = std::ptr::null_mut();
    adventerm_ffi::save_from_bytes(buf.as_ptr(), buf.len(), &mut restored);

    let mut restored_game: *mut adventerm_ffi::GameHandle = std::ptr::null_mut();
    adventerm_ffi::save_to_game(restored, &mut restored_game);

    // Compare positions
    let mut x1 = 0usize; let mut y1 = 0usize;
    let mut x2 = 0usize; let mut y2 = 0usize;
    adventerm_ffi::game_player_pos(h, &mut x1, &mut y1);
    adventerm_ffi::game_player_pos(restored_game, &mut x2, &mut y2);
    assert_eq!((x1, y1), (x2, y2));

    adventerm_ffi::save_free(save_handle);
    adventerm_ffi::save_free(restored);
    adventerm_ffi::game_free(h);
    adventerm_ffi::game_free(restored_game);
}

#[test]
fn save_from_bytes_unsupported_version() {
    let bad = br#"{"version": 999, "name": "x", "state": null}"#;
    let mut out: *mut adventerm_ffi::SaveHandle = std::ptr::null_mut();
    let rc = adventerm_ffi::save_from_bytes(bad.as_ptr(), bad.len(), &mut out);
    assert_eq!(rc, adventerm_ffi::FfiError::SaveFormat as i32);
    // (or UnsupportedSaveVersion if serde gets past the schema check)
    assert!(out.is_null());
}

#[test]
fn list_saves_in_tempdir() {
    let dir = tempfile::tempdir().unwrap();
    // Write two saves via FFI, then list and assert count == 2
}
```
