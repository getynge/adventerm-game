# adventerm_ffi

C-compatible FFI surface over [`adventerm_lib`](../adventerm_lib/). Lets non-Rust hosts (Swift on iOS, JNI on Android, plain C on desktop) drive a full game session: construct, dispatch actions, query state, save/load, run battles, and drive the dev console.

The Rust-first rule (CLAUDE.md rule #2 generalized) means `adventerm_lib` exposes idiomatic Rust and `adventerm_ffi` does the shim work. Trait objects, generics, lifetimes, and `Vec<T>`/`HashMap` never appear in `extern "C"` signatures.

## 1. Crate location and crate-types

Crate root: [adventerm_ffi/](../adventerm_ffi/). `Cargo.toml` declares three crate-types — each serves a distinct consumer:

| Crate-type | Built artifact | Used by |
| --- | --- | --- |
| `staticlib` | `libadventerm_ffi.a` | iOS XCFramework, Android NDK static link |
| `cdylib` | `libadventerm_ffi.{so,dylib,dll}` | Linux/macOS/Windows desktop hosts, JNI |
| `rlib` | `libadventerm_ffi.rlib` | Rust integration tests in [adventerm_ffi/tests/](../adventerm_ffi/tests/) |

The `rlib` is the cheap path: every test under `tests/` links directly against the FFI exports without going through a C compiler. The C smoke test ([tests/smoke.c](../adventerm_ffi/tests/smoke.c)) is the only consumer that actually crosses the C ABI in CI.

## 2. Handle conventions

Handles are opaque newtypes in [src/handle.rs](../adventerm_ffi/src/handle.rs): `GameHandle`, `BattleHandle`, `SaveHandle`, `SaveListing`, `ConsoleHandle`. Each is `pub struct H { pub(crate) inner: ... }`. **No `#[repr(C)]`** — cbindgen emits opaque forward declarations because the C side never introspects layout.

Rules:

- **Alloc/free pairs.** Every `*_new[_*]` (or constructor that returns a handle) is balanced by exactly one `*_free`. Listed in `agents/ffi.md` § 1 of each topic module ([game.rs](../adventerm_ffi/src/game.rs), [battle.rs](../adventerm_ffi/src/battle.rs), [save.rs](../adventerm_ffi/src/save.rs), [console.rs](../adventerm_ffi/src/console.rs)).
- **Null-safe free.** `*_free(NULL)` is a no-op (matches C's `free(NULL)`). See [game.rs:26](../adventerm_ffi/src/game.rs) for the canonical pattern.
- **No parent/child borrows.** Sub-handles (e.g. `BattleHandle`, `SaveHandle`) are independent owners. Construction clones or moves the lib-side data so the new handle survives the parent's `*_free`.
- **No interior pointers.** No FFI function returns a pointer that aliases a handle's interior. Strings copy out into caller-allocated buffers; small structs return by value.
- **Single-threaded per handle.** The caller synchronizes. The crate adds no `Send`/`Sync` interlocks. `LAST_ERROR` is thread-local, so each thread sees its own error stash.

Buffer outputs follow the two-call pattern: pass `buf == NULL` with `out_required != NULL` to discover the size, then allocate and call again. See [save.rs:28](../adventerm_ffi/src/save.rs) (`copy_into_buf`) for the shared helper.

## 3. Error codes and `LAST_ERROR`

Source of truth: [src/error.rs](../adventerm_ffi/src/error.rs). Every fallible `extern "C"` returns `i32` whose value is one of the [`FfiError`](../adventerm_ffi/src/error.rs) discriminants.

| Variant | Code | Meaning |
| --- | --- | --- |
| `Ok` | `0` | success |
| `NullArgument` | `-1` | required pointer was null |
| `InvalidUtf8` | `-2` | `*const c_char` input was not valid UTF-8 |
| `OutOfRange` | `-3` | numeric argument outside allowed range (slot index, room id, etc.) |
| `EmptySlot` | `-4` | ability slot was empty when invoked |
| `NotPlayerTurn` | `-5` | tried to apply player ability during enemy turn |
| `AlreadyResolved` | `-6` | tried to advance a resolved battle |
| `NoSuchEntity` | `-7` | `EntityId` does not refer to a live entity |
| `NoPendingEncounter` | `-8` | tried to consume an encounter when none is pending |
| `SaveFormat` | `-9` | `Save::from_bytes` rejected JSON shape |
| `UnsupportedSaveVersion` | `-10` | save version did not match `SAVE_VERSION` |
| `IoFailure` | `-11` | `std::io::Error` from filesystem helpers |
| `BufferTooSmall` | `-12` | caller buffer too small (see two-call pattern) |
| `InternalPanic` | `-13` | a panic crossed `ffi_try!`; payload in `LAST_ERROR` |
| `Unknown` | `-99` | reserved sentinel |

Discriminants are stable wire codes — append only, never reorder or repurpose. Positive values are reserved for future use.

`LAST_ERROR` is a `thread_local!` `RefCell<Option<CString>>` ([src/error.rs:36](../adventerm_ffi/src/error.rs)). `From` impls populate it with detail (file path, parse offset, panic payload). Read it via:

```c
const char* msg = ffi_last_error_message();  // NULL if no detail set
```

The pointer is valid until the next FFI call on the same thread; copy immediately if you need to keep the message past that point.

The [`ffi_try!`](../adventerm_ffi/src/error.rs) macro wraps every fallible body in `catch_unwind`, converts panics to `InternalPanic`, and stashes the payload. Every fallible export uses it — no exceptions.

## 4. Header location and regeneration

Generated header: [adventerm_ffi/include/adventerm_ffi.h](../adventerm_ffi/include/adventerm_ffi.h). **Committed to the repo** so consumers (Swift Package, smoke tests, downstream CI) get a stable artifact without running `cbindgen`.

Generation pipeline:

1. [build.rs](../adventerm_ffi/build.rs) runs on every `cargo build -p adventerm_ffi`.
2. It reads [cbindgen.toml](../adventerm_ffi/cbindgen.toml) and walks the FFI crate's own `src/` (with `parse.parse_deps = false`, so `adventerm_lib` types stay invisible).
3. Output is written to `include/adventerm_ffi.h` verbatim.

CI guarantees regeneration is a no-op:

```bash
cargo build -p adventerm_ffi
git diff --exit-code adventerm_ffi/include/
```

A non-empty diff fails the PR. Re-run the build, commit the regenerated header alongside the FFI source change, push.

The header is additionally syntax-checked standalone:

```bash
clang -Wall -Wpedantic -fsyntax-only -x c \
    -Iadventerm_ffi/include adventerm_ffi/include/adventerm_ffi.h
```

## 5. Per-platform build

Desktop / Android (single target):

```bash
cargo build --release -p adventerm_ffi --target <triple>
```

Triples in CI: `x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`, `aarch64-apple-darwin`, `x86_64-apple-darwin`, `aarch64-linux-android`, `armv7-linux-androideabi`. Android targets need an NDK install plus per-target linker config; iOS triples need Xcode and `rustup target add`.

iOS XCFramework (release-only, manually invoked):

```bash
adventerm_ffi/scripts/build-xcframework.sh
```

The script ([scripts/build-xcframework.sh](../adventerm_ffi/scripts/build-xcframework.sh)):

1. `cargo build --release` for `aarch64-apple-ios`, `aarch64-apple-ios-sim`, `x86_64-apple-ios`.
2. `lipo -create` the two simulator slices into `target/ios-sim-fat/release/libadventerm_ffi.a`.
3. `xcodebuild -create-xcframework` bundles device + sim-fat into `adventerm_ffi/build/AdventermFFI.xcframework`.

The XCFramework is not produced by per-PR CI — only by the release job. Per-PR CI compiles each iOS triple individually (sufficient to catch ABI breakage).

## 6. Swift consumer guide

### Wrap a handle in a Swift class

The C side gives you `OpaquePointer`-shaped handles. Wrap them in a Swift class so ARC frees them deterministically:

```swift
final class Game {
    fileprivate let handle: OpaquePointer

    init(seed: UInt64) {
        guard let h = game_new_seeded(seed) else {
            fatalError("game_new_seeded returned NULL")
        }
        self.handle = h
    }

    deinit { game_free(handle) }
}
```

One handle, one class. `deinit` is the only place that calls `*_free`. If a method needs to hand a sub-handle to another Swift class, that other class owns the sub-handle and frees it in its own `deinit` (no parent/child lifetime contracts — see § 2).

### Two-call buffer pattern

Most string and byte outputs use the discover-then-fill pattern. A small extension makes it idiomatic:

```swift
extension Game {
    func saveBytes(_ save: Save) throws -> Data {
        var needed: Int = 0
        var rc = save_to_bytes(save.handle, nil, 0, &needed)
        guard rc == FfiError.bufferTooSmall.rawValue else {
            throw FfiError(rawValue: rc) ?? .unknown
        }
        var buf = Data(count: needed)
        rc = buf.withUnsafeMutableBytes { raw in
            save_to_bytes(save.handle,
                          raw.baseAddress!.assumingMemoryBound(to: UInt8.self),
                          needed, &needed)
        }
        guard rc == 0 else { throw FfiError(rawValue: rc) ?? .unknown }
        return buf
    }
}
```

The first call always returns `BufferTooSmall` (`-12`) and writes the required size. The second call writes the bytes.

### Map `FfiError` to `enum: Error`

```swift
enum FfiError: Int32, Error {
    case ok = 0
    case nullArgument = -1
    case invalidUtf8 = -2
    case outOfRange = -3
    case emptySlot = -4
    case notPlayerTurn = -5
    case alreadyResolved = -6
    case noSuchEntity = -7
    case noPendingEncounter = -8
    case saveFormat = -9
    case unsupportedSaveVersion = -10
    case ioFailure = -11
    case bufferTooSmall = -12
    case internalPanic = -13
    case unknown = -99

    var detail: String? {
        guard let p = ffi_last_error_message() else { return nil }
        return String(cString: p)  // copy now — pointer is valid until next FFI call
    }
}
```

Translate `i32` returns at every call site: `0` is success, anything else throws (or maps to a Result). Read `detail` immediately after — the pointer is invalidated by the next FFI call on this thread.

## 7. What does not cross the FFI boundary

Hard list, derived from the Rust-first rule:

- **Trait objects** — `&dyn ItemBehavior`, `&dyn ActiveAbility`, `&dyn DevCommand`, `&dyn EnemyAi`. Behavior is selected lib-side by enum discriminant; only outcomes are visible to C.
- **Raw `Vec<T>` / `HashMap<K, V>` / `String`** — exposed via copy-into-buffer (Pattern B in [plans/ffi/02-error-and-strings.md](../plans/ffi/02-error-and-strings.md)) or count+index iteration (Pattern A in [plans/ffi/05-iteration-and-save.md](../plans/ffi/05-iteration-and-save.md)).
- **Lifetimes and generics** — every FFI signature is concrete; no `<T>`, no `'a`. Borrowed `'static` strings use `OnceLock<CString>` interning lib-side.
- **ECS internals** — `World`, `ComponentStore<T>`, the per-room subsystems (`Lighting`, `ItemSubsystem`, `Enemies`, `Abilities`, `DoorSubsystem`, `BattleSubsystem`). Hosts read gameplay state through `GameHandle` facades, never the substrate.
- **Event bus** — `EventBus`, `ErasedEvent`, `Registry`, `TickLog`. Hosts observe outcomes via per-action return values, not by subscribing.
- **`EntityId` is opaque.** It crosses the boundary as a `u32` (`EntityId::raw()` / `EntityId::from_raw(u32)`) but is never inspected on the C side — pass it back unchanged.

If you find yourself wanting to expose any of the above, the answer is to add a focused FFI shim that produces the data the host actually needs.

## 8. Stability rules

- **Enum discriminants are wire-stable. APPEND ONLY.** [`FfiError`](../adventerm_ffi/src/error.rs) values, every `CXxx` mirror enum in [enums.rs](../adventerm_ffi/src/enums.rs), and every tagged-struct `tag` field in [structs.rs](../adventerm_ffi/src/structs.rs) are part of the public ABI. Adding a new variant goes at the end of the enum. Reordering or repurposing a discriminant is a hard break.
- **Function signatures are wire-stable per release.** Renames or signature changes require a major version bump and a coordinated header roll. Adding a new export is non-breaking.
- **`SAVE_VERSION` bumps invalidate older saves on the FFI side identically to the Rust side.** `save_from_bytes` returns `UnsupportedSaveVersion` (`-10`) with detail "found N, expected M" in `LAST_ERROR`. The lib-side rule (see [agents/library.md](library.md) § save.rs) is the source of truth — the FFI just propagates.
- **Header diffs gate every PR.** If `cargo build -p adventerm_ffi` modifies `adventerm_ffi/include/adventerm_ffi.h`, the regenerated header must be committed in the same PR. CI enforces.
