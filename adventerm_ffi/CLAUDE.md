# adventerm_ffi

C-compatible FFI surface over `adventerm_lib`. Lets non-Rust hosts (Swift on iOS, JNI on Android, plain C on desktop) drive a full game session: construct, dispatch actions, query state, save/load, run battles, drive the dev console.

## Hard rules

1. **Rust-first stays.** `adventerm_lib` keeps its idiomatic Rust API. This crate adds no `#[repr(C)]` to lib types, no `extern "C"` exports in lib modules, no `*mut`/`*const` in lib signatures. The shim layer lives entirely here.
2. **Trait objects don't cross.** `&dyn ItemBehavior`, `&dyn ActiveAbility`, `&dyn DevCommand`, `&dyn EnemyAi` never appear in FFI signatures. Behavior is selected lib-side by enum discriminant; only outcomes are visible to C.
3. **Every fallible export uses `ffi_try!`.** The macro wraps the body in `catch_unwind`, converts panics to `FfiError::InternalPanic`, and stashes the payload in `LAST_ERROR`. No exceptions. See [src/error.rs](src/error.rs).
4. **Opaque handles only.** Big things are `*mut <Handle>` newtypes ([src/handle.rs](src/handle.rs)); small `Copy` data goes by value; strings use the two-call buffer pattern with `out_required`. Never return a borrowed pointer into a handle's interior.
5. **Single-threaded per handle.** `LAST_ERROR` is thread-local; callers synchronize their own handles. No `Send`/`Sync` interlocks here.
6. **Header is generated, then committed.** [build.rs](build.rs) runs `cbindgen` against [cbindgen.toml](cbindgen.toml) and writes [include/adventerm_ffi.h](include/adventerm_ffi.h). CI fails the PR if `git diff --exit-code adventerm_ffi/include/` is non-empty after `cargo build -p adventerm_ffi`.

## Reference

Full conventions, error table, Swift consumer guide, and stability rules live in [agents/ffi.md](../agents/ffi.md). Read that before adding a new export, a new handle type, or a new error variant.

When syncing this crate after an `adventerm_lib` public-API change, run the [sync-ffi](../.claude/skills/sync-ffi/SKILL.md) skill (or `/sync-ffi`). Inline is fine for small mirror changes; consider a subagent (`Agent` tool, `subagent_type: general-purpose`) running the same skill for broad changes where build/test churn would clutter the main context.

## Commands

- Build: `cargo build -p adventerm_ffi`
- Test: `cargo test -p adventerm_ffi`
- iOS XCFramework (manual / release CI): `adventerm_ffi/scripts/build-xcframework.sh`
