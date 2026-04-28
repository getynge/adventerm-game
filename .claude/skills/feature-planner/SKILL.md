---
name: feature-planner
description: Plan a large or cross-cutting feature end-to-end before any code is written. Use when the user asks to design, plan, scope, or scaffold a feature that spans multiple modules or crates, when they say "plan a feature", "design X", "scope Y", or invoke /feature-planner. Produces a phased plan under plans/<slug>/.
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

# feature-planner — Plan a feature end-to-end before writing code

This skill walks a feature from "rough idea" to a phased, durable plan committed under `plans/<slug>/`. It is the entry point for any change large enough that diving straight to code would lose intent or skip a hard rule from [CLAUDE.md](../../../CLAUDE.md).

The output is a directory of markdown files modelled on [plans/ffi/](../../../plans/ffi/): an `00-overview.md` plus one `NN-<phase>.md` per phase. Future sessions implement from those files.

Read [agents/README.md](../../../agents/README.md) before § 2 — it indexes which deep doc applies to which area of the codebase.

---

## 0. Operating principles

These govern every later step. They are not negotiable; surface conflicts to the user instead of working around them.

- **Do not assume unstated intent.** Any decision the user has not made explicit is a knowledge gap that § 5 must close. The number of clarifying questions can be 0 (the user pre-stated everything) or 100+ (the feature is large and ambiguous). Err toward more, never fewer.
- **Read current code; do not rely on memory.** `agents/` docs and `CLAUDE.md` are the source of truth for architecture; `git log` and the working tree are the source of truth for recent state.
- **Stop at every `CLAUDE.md` hard rule.** The lib/binary boundary, append-only enum discriminants, FFI sync requirement, no `ratatui`/`crossterm` in `adventerm_lib`, no UI state in the lib — these are absolute. If the feature appears to require breaking one, raise it with the user before writing the plan.
- **Reuse existing helpers.** Check [agents/patterns.md](../../../agents/patterns.md) before introducing a new abstraction. Cite the helper's path in the plan when it is reused.
- **Phase boundaries are PR-sized.** Each `NN-<phase>.md` should describe work that fits in one reviewable PR with its own verification.
- **Plan files contain the recommended approach only.** Not a menu of alternatives. Decisions go in `00-overview.md`'s decisions matrix with the user's reasoning attached.
- **Plan mode interaction.** § 6 writes files, so the skill must finish in normal mode. If the user invoked `/feature-planner` from plan mode, complete §§ 1–5 read-only and prompt the user to exit plan mode before § 6.

---

## 1. Get the high-level feature summary

Plain-text prompt to the user. Do **not** propose architecture, name modules, or sketch types yet — those steps depend on § 2's grounding.

Capture:

- What the feature does, in their own words.
- Who it is for (player-facing, dev-facing via the console, host-app via FFI).
- The motivating problem or absence the feature addresses.

Wait for the reply before continuing.

## 2. Build and present the current architecture

In parallel via the `Read` tool, load:

- [/Users/getynge/code/adventerm-game/CLAUDE.md](../../../CLAUDE.md) — workspace rules.
- [adventerm_lib/CLAUDE.md](../../../adventerm_lib/CLAUDE.md), [adventerm/CLAUDE.md](../../../adventerm/CLAUDE.md), [adventerm_ffi/CLAUDE.md](../../../adventerm_ffi/CLAUDE.md) — per-crate rules.
- [agents/README.md](../../../agents/README.md), [agents/architecture.md](../../../agents/architecture.md), [agents/patterns.md](../../../agents/patterns.md) — always relevant.

Then load the deep doc(s) that match the feature's apparent area:

| Apparent area | Deep doc |
| --- | --- |
| Gameplay rule, dungeon, save, ECS, behavior trait | [agents/library.md](../../../agents/library.md) |
| Screen, menu, keybind, color scheme, rendering | [agents/tui.md](../../../agents/tui.md) |
| FFI export, handle, error code, Swift consumer | [agents/ffi.md](../../../agents/ffi.md) |

For broad or uncertain scope, spawn 1–3 `Explore` subagents **in parallel** — each on a distinct area (lib, binary, FFI) or a distinct concern (existing patterns, related screens, similar past features).

Synthesize a tailored 200–500 word architecture summary covering:

- Workspace topology (the three crates and what each owns).
- The systems the feature most likely touches, with the specific facades and types involved (e.g. `Room::items_at`, `GameState::dispatch`, `Save::to_bytes`).
- Current interaction patterns — how data flows lib → binary → FFI for the relevant slice.
- The hard rules from `CLAUDE.md` that constrain this feature.

Present the summary inline. Ask the user to correct misunderstandings before continuing. **Treat their corrections as authoritative** — they know the codebase better than the docs do.

## 3. Ask how the feature fits in

Anchored on § 2's summary. Ask, in prose:

- Which crate(s) own the new code (lib only / lib + binary / lib + binary + ffi / ...).
- Which existing modules and facades it extends or calls.
- Where it sits in the screen FSM (UI features) or the action / event pipeline (gameplay features).
- Whether it crosses the FFI boundary (and therefore triggers [sync-ffi](../sync-ffi/SKILL.md)).

Use `AskUserQuestion` for clean 2–4-option choices. Use prose for open-ended placement questions.

## 4. Ask for key guidelines

Open prose ask. Cover at minimum:

- Scope: MVP vs full implementation. Cutlines.
- Style preferences and what to avoid (matches against project [CLAUDE.md](../../../CLAUDE.md) and global standards).
- Performance budget, if any.
- Telemetry, logging, dev-console integration.
- Save-format compatibility — does this need a `SAVE_VERSION` bump?
- FFI implications — new exports, new error codes, new handle types.
- Accessibility — color-scheme entries, keybinds, screen-reader-relevant text.

Note any guideline that contradicts a `CLAUDE.md` rule and surface the conflict before continuing.

## 5. Identify knowledge gaps and clarify

The load-bearing step. Enumerate every unstated decision the feature implies. Examples of categories that almost always have gaps:

- **Data shape** — field names, types, units, ranges, allowed values, defaults.
- **Error handling** — failure modes, what propagates vs is recovered, user-facing messages.
- **Edge cases** — empty inputs, max sizes, concurrent state changes, save/load round trips.
- **Naming** — public type / function / variant names, file names, slug.
- **Persistence** — does it serialize, in which struct, behind which `SAVE_VERSION`.
- **State ownership** — which subsystem owns the new state; UI-only state stays in the binary (rule #3).
- **RNG / generation** — seeded vs unseeded, deterministic across saves.
- **Input bindings** — keybinds, accelerator letters, conflicts with existing menus.
- **Color scheme entries** — new tokens in [adventerm/schemes/](../../../adventerm/schemes/) and [src/ui/colors.rs](../../../adventerm/src/ui/colors.rs).
- **FFI mirror** — which lib symbols cross, new error variants, handle lifecycle.

Group related ambiguities and ask via `AskUserQuestion` in batches of 2–4 questions per call. Iterate — each answer can surface new questions — until the open list is empty.

Track every decision internally with the user's reasoning attached. § 6 will quote each one back in the decisions matrix.

## 6. Slug, phase split, and file output

**Slug.** Derive a kebab-case slug from the feature title. If ambiguous, ask via `AskUserQuestion` with two or three sensible options.

**Directory.** `mkdir -p plans/<slug>/`. If the directory already has files, ask whether to overwrite, append, or pick a new slug.

**Phase count** scales with feature size:

- Trivial / single touchpoint: one file, `plans/<slug>/01-<phase>.md`.
- Standard: 2–4 phases.
- Large / cross-crate: 5+ phases. Mirror [plans/ffi/](../../../plans/ffi/)'s 7-milestone shape — each phase is a PR.

**`00-overview.md`** always contains, in order:

1. **Context** — why this change is being made; the problem and intended outcome.
2. **Goal** — one-paragraph statement of what done looks like.
3. **Hard constraints** — the `CLAUDE.md` rules this feature must respect, restated.
4. **Decisions matrix** — table of every clarified gap → its decision, with the user's reasoning where given. Mirror the table style in [plans/ffi/00-overview.md](../../../plans/ffi/00-overview.md).
5. **Phase summary table** — one row per phase: title, plan file link, focal files, verification command.
6. **Out of scope** — what is intentionally deferred.
7. **Reused existing helpers** — file paths from [agents/patterns.md](../../../agents/patterns.md) and the codebase that this plan leans on instead of writing fresh code.

**`NN-<phase-slug>.md`** for each phase contains:

- Scope of the phase (one paragraph).
- Files to touch (repo-relative paths). New files: where and why.
- Code sketches (type signatures, public-API outlines) only where they earn their keep.
- Per-phase verification — exact commands to run, tests to add, manual checks.

Reference `agents/` docs and `CLAUDE.md` by relative path, exactly as `sync-ffi` does.

## 7. Final report

Summarize for the user:

- Every file written, with its path.
- One-line per phase: title and verification.
- Anything that touched a `CLAUDE.md` hard rule and needs a design discussion before implementation begins.
- Whether the feature crosses the FFI boundary (and therefore the implementation phase will need to invoke [sync-ffi](../sync-ffi/SKILL.md)).

Do not start implementing. The skill ends with the plan committed and the user free to schedule the work.
