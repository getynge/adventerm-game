---
name: refactor-planner
description: Plan a behavior-preserving refactor of existing exploration-phase code end-to-end before any code is moved. Use when the user asks to clean up, tidy, restructure, or refactor a named target (file, module, subsystem) — "refactor X", "clean up Y", "tidy Z", or invokes /refactor-planner. Produces a phased plan under plans/<slug>/ where every phase keeps cargo test green.
user-invocable: true
allowed-tools:
  - Read
  - Write
  - Edit
  - Bash(rg *)
  - Bash(ls *)
  - Bash(find *)
  - Bash(wc *)
  - Bash(mkdir *)
  - Bash(git log*)
  - Bash(git diff*)
  - Bash(git show*)
  - Bash(git status*)
---

# refactor-planner — Plan a refactor end-to-end before moving code

Sister skill to [feature-planner](../feature-planner/SKILL.md). Where that one designs new behavior, this one redesigns existing behavior's *shape*: turning exploration-phase code — god structs, magic numbers, ad-hoc helpers, facade-bypassing call sites — into code that follows [CLAUDE.md](../../../CLAUDE.md) and reuses the helpers in [agents/patterns.md](../../../agents/patterns.md).

The output is a directory of markdown files modelled on [plans/ffi/](../../../plans/ffi/): an `00-overview.md` plus one `NN-<phase>.md` per phase. Every phase is one PR-sized step that keeps `cargo test` green; future sessions implement from those files.

Read [agents/README.md](../../../agents/README.md) before § 2 — it indexes which deep doc applies to which area of the codebase.

---

## 0. Operating principles

These govern every later step. They are not negotiable; surface conflicts to the user instead of working around them.

- **Refactors must not change behavior.** A refactor that rearranges code without altering observable outputs. If a phase requires a behavior change (different ordering, different output, different error path), surface it to the user — that is a feature change, and may belong in [feature-planner](../feature-planner/SKILL.md) instead.
- **Read current code; do not rely on memory.** `agents/` docs and `CLAUDE.md` describe the architecture; the working tree describes the actual shape today. They diverge whenever exploration code outpaces the docs — trust the tree.
- **Stop at every `CLAUDE.md` hard rule.** A refactor is the *right* time to bring violations back into line (lib/binary boundary, no `ratatui`/`crossterm` in the library, append-only enum discriminants, FFI sync requirement, no UI state in the lib) — but never to introduce a *new* violation.
- **Reuse before invent.** Every replaced ad-hoc helper must cite an existing helper from [agents/patterns.md](../../../agents/patterns.md) by path. If no helper fits and a new one is justified, name it explicitly in the plan with its proposed home and rationale.
- **Phase boundaries are PR-sized and behavior-preserving.** Each `NN-<phase>.md` describes work that fits in one reviewable PR and ends with `cargo test` green. Never plan a phase that leaves the workspace red and counts on the next phase to fix it.
- **Plan files contain the recommended approach only.** Not a menu of alternatives. Decisions go in `00-overview.md`'s decisions matrix with the user's reasoning attached.
- **Plan-mode interaction.** § 6 writes files, so the skill must finish in normal mode. If the user invoked `/refactor-planner` from plan mode, complete §§ 1–5 read-only and prompt the user to exit plan mode before § 6.

---

## 1. Get the refactor target

Plain-text prompt to the user. Do **not** propose a target shape, name modules, or sketch types yet — those depend on § 2's grounding.

Capture:

- The target — which file, module, subsystem, or call-site cluster.
- The discomfort — in their words. Common shapes: god struct, magic numbers, duplicated rendering / dispatch logic, lib/binary boundary leak, inlined helper that earns its own name, mixed concerns in one function, dead code.
- Hard out-of-scope — anything they explicitly do not want touched ("don't change the save format", "leave the keybinds alone", "the FFI surface is frozen").

Wait for the reply before continuing.

## 2. Read the target and the rules

In parallel via the `Read` tool, load:

- [/Users/getynge/code/adventerm-game/CLAUDE.md](../../../CLAUDE.md) — workspace rules.
- [adventerm_lib/CLAUDE.md](../../../adventerm_lib/CLAUDE.md), [adventerm/CLAUDE.md](../../../adventerm/CLAUDE.md), [adventerm_ffi/CLAUDE.md](../../../adventerm_ffi/CLAUDE.md) — per-crate rules.
- [agents/README.md](../../../agents/README.md), [agents/architecture.md](../../../agents/architecture.md), [agents/patterns.md](../../../agents/patterns.md) — always relevant.

Then load the deep doc(s) that match the target:

| Target area | Deep doc |
| --- | --- |
| Gameplay rule, dungeon, save, ECS, behavior trait | [agents/library.md](../../../agents/library.md) |
| Screen, menu, keybind, color scheme, rendering | [agents/tui.md](../../../agents/tui.md) |
| FFI export, handle, error code, Swift consumer | [agents/ffi.md](../../../agents/ffi.md) |

Also load the target file(s) themselves in full. For a broad target ("clean up the binary", "tidy the inventory subsystem"), spawn 1–3 `Explore` subagents **in parallel** — one per crate, or one per concern (call-site map of the target's public symbols, existing helpers nearby that the target ignores, current tests that exercise the target).

Synthesize a tailored 200–500 word *current-shape* summary covering:

- What the target does, in one or two sentences.
- Who calls it (file paths, with line numbers where load-bearing).
- What state it owns vs. what it borrows.
- Which facades from [agents/patterns.md](../../../agents/patterns.md) it bypasses (cite each with the table-row helper that should replace the inlined version).
- Which `CLAUDE.md` rules it currently violates, if any.

Present the summary inline. Ask the user to correct misunderstandings before continuing. **Treat their corrections as authoritative** — they know the codebase better than the docs do.

## 3. Identify the smells

Concrete table per target. One row per smell. Columns:

| # | Smell | File:line | Replacement |
|---|-------|-----------|-------------|

Smell categories to look for:

- **God struct** — fields that belong to different lifetimes, owners, or screens cohabiting one type. Replacement: split per-screen state to the screen variant that uses it (CLAUDE.md style rule #1 + [agents/architecture.md](../../../agents/architecture.md)'s screen FSM).
- **Magic number** — inline literal for a size, padding, range, duration, or capacity. Replacement: named constant in the file's existing constants block, or in [adventerm/src/ui/layout.rs](../../../adventerm/src/ui/layout.rs) for layout literals (CLAUDE.md style rule #2).
- **Duplicated logic** — the same five lines in two or three call sites. Replacement: extract to a helper next to the existing peers, or pull from [agents/patterns.md](../../../agents/patterns.md) if a peer already exists (`accel::line`, `popup_rect`, `slugify`, …).
- **Boundary leak** — `KeyCode`, `Color`, `Rect`, or any `ratatui`/`crossterm` type in the library; or `World`, `Dungeon`, `Lighting`, `ItemSubsystem` reached for from the binary. Replacement: route through the existing facade (`GameState::tile_at`, `peek_item_here`, …) or add a new lib facade if none fits (CLAUDE.md architectural rule #2).
- **Inlined helper** — a non-trivial computation expressed inline at a call site that has its own name in the team's vocabulary. Replacement: small named function near the call site (CLAUDE.md style rule #3).
- **Mixed concerns** — input translation alongside rendering, save logic alongside gameplay, etc. Replacement: split along the existing module seam (`input.rs` for translation, `save.rs` for persistence, …).
- **Dead code** — unreachable branches, unused fields, commented-out blocks, vestiges of earlier exploration. Replacement: delete (after confirming no FFI / save consumer depends on the symbol).

Cite [agents/patterns.md](../../../agents/patterns.md) entries by path when the replacement is a documented helper — that is the standing list of "use this before writing your own".

Present the table inline. Ask the user which smells are in scope vs. deferred. The deferred set goes into `00-overview.md`'s **Out of scope** section verbatim.

## 4. Ask for refactor guidelines

Open prose ask. Cover at minimum:

- **Cutlines.** MVP refactor (one or two smells) vs. full cleanup (every smell named). Where to stop.
- **Name stability.** Public lib symbols may have non-Rust consumers via [adventerm_ffi](../../../adventerm_ffi/) — renames there cascade through the FFI mirror types and the regenerated header. Ask the user to flag any name that must stay stable.
- **Save-format compatibility.** Renaming, reordering, or restructuring any type involved in `Save::to_bytes` / `Save::from_bytes` may change the JSON shape. Default: must remain `SAVE_VERSION`-compatible. If a bump is acceptable, the user must say so explicitly.
- **FFI implications.** Does the refactor rename, remove, or restructure a public lib symbol that crosses (or could cross) the FFI? If yes, the implementation phase will end with [sync-ffi](../sync-ffi/SKILL.md). Note this in the plan so the implementer cannot miss it.
- **Test coverage gaps.** Which behavior is *not* covered by existing tests but will be moved by this refactor? Those gaps need characterization tests added *before* the risky moves — they are the only safety net for behavior preservation in untested code paths.
- **Style preferences.** Anything in the user's [global CLAUDE.md](file:///Users/getynge/.claude/CLAUDE.md) or the project's [CLAUDE.md](../../../CLAUDE.md) that the current code violates and the refactor should restore.

Note any guideline that contradicts a `CLAUDE.md` rule and surface the conflict before continuing.

## 5. Identify knowledge gaps and clarify

The load-bearing step. Refactors hinge on naming and placement decisions that the user has opinions about. Enumerate every unstated decision.

Categories that almost always have gaps:

- **Smell-by-smell scope.** Which smells from § 3's table are addressed *now* vs. left for a follow-up.
- **Helper naming.** What to call extracted functions, types, modules, fields.
- **Helper placement.** Where extracted helpers live (existing module vs. new file; near callers vs. near related helpers).
- **Module structure.** Whether a target file gets split into a directory of submodules; whether two sibling files merge.
- **Field grouping.** When breaking up a god struct, which fields cluster into which sub-struct.
- **Dead-code disposition.** Delete now, or move to a `// TODO: remove after X` comment with a follow-up issue.
- **Test placement.** Inline `#[cfg(test)] mod tests` vs. `tests/` integration; characterization-test naming.
- **Phase sequencing.** Which order to do things in. Almost always: (a) add characterization tests, (b) extract helpers without changing call sites, (c) migrate call sites, (d) delete the old shape. Confirm this default with the user before locking it in.

Group related ambiguities and ask via `AskUserQuestion` in batches of 2–4 questions per call. Iterate — each answer can surface new questions — until the open list is empty.

Track every decision internally with the user's reasoning attached. § 6 will quote each one back in the decisions matrix.

## 6. Slug, phase split, and file output

**Slug.** Derive a kebab-case slug from the refactor target (e.g. `tidy-app-rs`, `split-inventory-renderer`, `pull-colors-through-scheme`). If ambiguous, ask via `AskUserQuestion` with two or three sensible options.

**Directory.** `mkdir -p plans/<slug>/`. If the directory already has files, ask whether to overwrite, append, or pick a new slug.

**Phase count** scales with refactor size:

- Trivial / single touchpoint: one file, `plans/<slug>/01-<phase>.md`.
- Standard: 2–4 phases.
- Large / cross-crate: 5+ phases. Mirror [plans/ffi/](../../../plans/ffi/)'s milestone shape — each phase is a PR.

A standard refactor sequence — adapt it to the smells in scope, do not paste it blindly:

1. **Characterization tests.** Add tests pinning behavior the refactor will move.
2. **Extract helpers.** New helpers exist alongside the old call sites; nothing migrates yet.
3. **Migrate call sites.** Old call sites switch to the new helpers, one cluster per phase if the surface is broad.
4. **Delete the old shape.** Remove vestigial fields, modules, and dead code.

**`00-overview.md`** always contains, in order:

1. **Context** — why this refactor is happening; the discomfort the user described and the intended outcome shape.
2. **Goal** — one-paragraph statement of what done looks like (the new shape, in prose).
3. **Hard constraints** — the `CLAUDE.md` rules being honored or restored, restated.
4. **Smells matrix** — the table from § 3, augmented with a `Phase #` column linking each in-scope smell to the phase that fixes it.
5. **Decisions matrix** — table of every clarified gap → its decision, with the user's reasoning where given. Mirror the table style in [plans/ffi/00-overview.md](../../../plans/ffi/00-overview.md).
6. **Phase summary table** — one row per phase: title, plan file link, focal files, verification command (`cargo test`, `cargo test -p <crate>`, `cargo test -p <crate> <test_name>`).
7. **Out of scope** — deferred smells (verbatim from § 3) and anything else explicitly punted.
8. **Reused existing helpers** — file paths from [agents/patterns.md](../../../agents/patterns.md) and the codebase that this plan leans on instead of writing fresh code.

**`NN-<phase-slug>.md`** for each phase contains:

- **Scope** — one paragraph; what the phase does and (just as importantly) what it deliberately does *not* do.
- **Files to touch** — repo-relative paths. New files: where and why.
- **Code sketches** — type signatures or function outlines *only* where the signature is the load-bearing detail (extracting a helper, renaming a type, splitting a struct). Skip otherwise.
- **Behavior preservation** — the safety net. Two parts:
  - *Existing tests that must stay green.* Names and paths (`adventerm_lib::room::tests::places_tile_correctly`, `tests/save_round_trip.rs`, …). The verification command runs these.
  - *Characterization tests added in this phase.* Only present in phases that introduce new tests. Name them, describe what behavior each pins down.
- **Per-phase verification** — exact commands to run. Always ends with the command that proves `cargo test` is green.

Reference `agents/` docs and `CLAUDE.md` by relative path, exactly as [feature-planner](../feature-planner/SKILL.md) and [sync-ffi](../sync-ffi/SKILL.md) do.

## 7. Final report

Summarize for the user:

- Every file written, with its path.
- One-line per phase: title and verification command.
- Any `CLAUDE.md` rule the refactor restores (e.g. "phase 3 removes the last `KeyCode` import from `adventerm_lib`, restoring architectural rule #2").
- Whether the refactor crosses the FFI boundary. If yes, the implementation phase will need to invoke [sync-ffi](../sync-ffi/SKILL.md) — surface this prominently so the implementer cannot miss it.

Do not start refactoring. The skill ends with the plan committed and the user free to schedule the work.
