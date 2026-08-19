# VERA20k — Engine Instructions

Shared project contract. `CLAUDE.md` and `AGENTS.md` point here; anything true of the engine
regardless of which tool is driving belongs here and nowhere else.

## What this project is

A from-scratch Rust engine that runs **Yuri's Revenge** — a faithful, playable replacement for
`gamemd.exe`.

Scale is the intentional exception to native storage limits: **20,000 units, 30 players**.
Replace any native structure that caps scale, preserving deterministic and player-visible
behavior.

## The delivery bar

> Every implemented behavior matches the verified `gamemd.exe` decompile semantics exactly.

Resolve ambiguous intent toward it. "Verified" is the Evidence standard below — the decompiled
body and its callsites actually read, this session or through a named research doc; exactness is
judged against that reading, never against intuition about what the original probably does. The
bar governs semantics, not structure: Rust-native architecture stands, and the charter's scale
replacement deliberately diverges internally while preserving deterministic and player-visible
behavior. Byte/pixel/frame/audio output equivalence follows the same rule — match it or record
the residual. Where exactness cannot yet be proven or afforded, the honest outcome is a
residual-named deferral, never an approximation absorbed as equivalent.

Player-visibility × frequency still ranks the work: fix what a player notices often first; edge
cases and ten-second screens wait while ordinary-play gaps remain. Ranking orders and defers
work — it never settles a verdict. "A player won't notice" is itself a claim requiring proof,
never a shortcut's justification. **The decompile is the spec; the production experience is how
you check it landed.**

## Evidence

Active YR `gamemd.exe`, retail INIs/assets, and observed production behavior are the reference.
Never guess when uncertainty could change common gameplay, deterministic state, authority,
lifecycle, persistence, commands, or shared architecture. Evidence scales in breadth, never in
standard — a localized fix needs no transitive-closure mapping of the binary, but every behavior
it lands still names its verified decompile source or an explicit UNCHECKED residual.

- **DRIFT is the default verdict** for any difference in formula, mechanism, ordering, field,
  byte, or render composition — not equivalent because it looks internal, rare, sub-pixel, or
  matched one sampled trace. Downgrade only with algebraic proof, a bit-identical test across the
  input space including boundaries, or exhaustive caller verification.
- **`VERIFIED` names a gamemd-derived executable check** ("verified by `test_x`") or exhaustive
  proof; otherwise `UNCHECKED`/`UNVERIFIED`. Prose never upgrades a status. This governs *claims*,
  not commits — ordinary fixes land with ordinary tests.
- **Goldens are machine-derived** — binary emulation, live capture, retail bytes. Hand-computed
  goldens have produced wrong references here. Rust-vs-prior-Rust hashes and replay fixtures are
  regression ratchets, not parity evidence.
- **Hand-maintained parity ledgers and completion trackers are forbidden** — they rot. Derive
  status from current code, git history, named checks, machine-generated evidence.
- A deferred DRIFT still gets its trigger, player effect, frequency and downstream risk recorded —
  never hidden, never called verified. It is non-deferrable when it repeatedly affects ordinary
  skirmish, changes outcomes or commands, breaks a loop or handoff, threatens deterministic state,
  or forces architectural rework; one-cell/pixel/frame/lepton/tick differences count when
  noticeable, frequent, compounding, or load-bearing.
- Scans and traces sample; a pass with no findings certifies nothing.
- **Every commit changing sim behavior names its gamemd source** — a live decompile citation, a
  named research doc, or an explicit "VERA-internal, gamemd equivalent UNCHECKED".
- State what was observed, what was tested, what remains unknown. Never declare exact victory from
  approximate evidence.

## Sources of truth

Order: `docs/research/` → live Ghidra decompilation → repo `ini/` → retail assets. Authority for
any conflict: **binary → Ghidra → docs**.

Skill instructions write `<main-checkout>` for the primary repository root. `docs/research/`,
`docs/plans/`, and the research-index *code* are tracked, so every worktree, clone, and fork
has them. Gitignored corpora — `ini/`, the rest of `docs/` (`scans/`, `gap-scans/`,
`contracts/`), and the built index `tools/research_index/.cache/` — exist only in the main
checkout; from a worktree, resolve it with `git worktree list`.

Grep the INI files before implementing; never hardcode animation names, frame counts, timing, or
constants. `rules(md).ini` = gameplay data, `art(md).ini` = all visual/animation data including
`Foundation=`. YR loads standalone `RULESMD.INI`/`ARTMD.INI`/`AIMD.INI` — it does *not* merge the
RA2 base INIs beneath them — then optional `LANGRULE.INI`, the mode INI, and the map INI in that
order. Use the in-repo `ini/`, never an external mod repo.

Inspect retail assets with the `asset` CLI (`cargo run -q --release --bin asset -- <verb>`,
`--help` lists verbs) or the `asset-browser` MCP server, not another one-off binary. Its palette
choice is inferred, so a plausible render is not evidence it is right; voxels render body-only;
`parse-check` "ok" means only that `from_bytes` returned Ok. Navigate research with the
`research-index` MCP server — top-N search ranks for triage and does not enumerate, so find the
anchor, expand, and read cited sections before editing. Check modification times for a parallel
session's in-progress output and extend it rather than duplicating.

**Do not create a research document unless code or a test lands in the same session.** Prose rots
silently while a test goes red. Convert a doc into a test only while already touching that system;
never run a corpus-wide cleanup pass.

## Native-to-Rust translation

**Rust-native structure, gamemd-native semantics.**

**The first question of every sim change — including bug fixes — is "what does gamemd do here?"**,
answered from research docs or live decompilation *before* designing the Rust. Never introduce a
gate, sentinel, clamp, fallback, or heuristic in `sim/` that gamemd lacks unless labeled
VERA-internal with the gamemd equivalent UNCHECKED. When debugging, the question is never "what
check would suppress this symptom".

Don't copy the C++ architecture literally — no raw pointer vectors, global mutable singletons,
COM/vtable plumbing, or the inheritance tree. Do copy the verified behavior contract carrying
player-visible or downstream meaning: ordering, state reads/writes, RNG consumption, timer
semantics, same-tick consequences, registration/removal.

- `EntityStore` owns storage; a scheduler owns active-object order; lifecycle helpers own reveal,
  conceal, limbo, unlimbo, uninit, delete, and scheduler registration effects.
- Plain functions implement behavior but commit state in verified native order.
- Recurring primitives — radio links, mission state machines, locomotor piggybacking, dock
  reservations, authority handoffs — are **modeled as mechanisms**, not replaced by constants for
  their common cases. Approximate locally only when the gamemd behavior has been read, the trigger
  is bounded, no deterministic or architectural debt is created, and the divergence is recorded as
  a deferred DRIFT against the bar — an approximation is a deferral, never an equivalent.
- Parallel helpers must be pure/read-only, or commit deterministically without changing
  player-visible ordering, RNG consumption, or same-tick visibility.

For native authority systems (`LogicClass`, `ObjectClass`, `TechnoClass`, `MissionClass`,
`RadioClass`, `MapClass`, `FactoryClass`, `HouseClass`), identify the behavior contract the
production loop needs, then map it to clean Rust — neither "C++ classes ported to Rust" nor "clean
Rust that visibly changes the game."

## Architecture boundaries

**The #1 invariant:** `sim/` must never depend on `render/`, `ui/`, `sidebar/`, `audio/`, or
`net/`. This enables headless servers, spectator views, and deterministic replay.

`assets/` + `util/` stay low-level and reusable; `rules/` + `map/` build data; `sim/` owns
deterministic state and gameplay; `render/`, `sidebar/`, `ui/`, `audio/`, `net/` sit above it; app
code orchestrates without absorbing gameplay logic. `src/lib.rs`, `src/sim/mod.rs`, and each
module's `//!` header are the source of truth for layout; the `SPINE REGION` / phase comments in
`World::advance_tick` are authoritative for tick order.

**Coordinates.** All `sim/` coordinates are **cell-grid `(u16, u16)`**, **+X = east, +Y = south**,
anchored at the building footprint NW corner. Porting an offset between the original's five
reference frames is a recurring bug class: name the source frame and unit (leptons ÷ 256, signed),
preserve conversion/shift/round/clamp semantics, and walk a concrete fixture through it before
trusting the math. Facing bytes (`0x00`=N, `0x40`=E, `0x80`=S, `0xC0`=W) are NOT drive-track curve
indices; isometric screen direction ≠ cell axes. Full reference:
`docs/research/coordinate-reference-frames.md`.

## Rust conventions

- Modules open with a `//!` doc comment: purpose and dependencies.
- Comment the *why*, not the what. Named constants for all magic numbers.
- Split files around ~600 lines when growth continues, except cohesive data-heavy files and tests.
- `thiserror` for library errors, `anyhow` for application propagation.
- **All sim math uses `fixed`-point types — never `f32`/`f64` in game logic.** Float is fine for
  rendering math in `glam`.
- `EntityStore` is `BTreeMap<u64, GameEntity>` — no ECS crate; deterministic sorted iteration is
  required for replay and lockstep.
- Asset parsers (.mix, .shp, .vxl, .pal, .tmp, .hva, .csf, .aud) are written from scratch with
  `nom`. No obscure or RA2-specific crates.
- `Cargo.toml` versions are pinned deliberately; don't upgrade without checking egui-wgpu
  compatibility.
- **Every Rust behavior whose semantics are derived in any way from `gamemd.exe` carries
  a nearby provenance comment.** Put it on the nearest owning item or behavior block; one
  comment may cover a cohesive implementation, while pure Rust architecture glue need not
  repeat it. It names the system/mechanism, the verified native class and function, and
  the exact Ghidra address. Never invent an identity to complete the format. Use the
  canonical one-line form and verified-identity fallback in
  `docs/research/ghidra-workflow.md`.

## Reverse engineering

Full rules — evidence discipline, label and save protocol, drift causes, the `param_1`
pointer-arithmetic pitfall, Tiberian Sun legacy — live in `docs/research/ghidra-workflow.md`. Read
it before any binary work. The rules that bind everywhere:

- **Asked to study, research, inspect, or investigate → analysis only**, no implementation unless
  explicitly requested.
- **Porting a system starts with a bounded mapping pass** — identify the functions and globals the
  port actually needs, and record evidence-backed annotation candidates before writing Rust. Do not
  expand to transitive closure. The mapped set is the port's scope; synchronizing that set into
  Ghidra follows the authorization rule below.
- **Ghidra annotation is candidate-first, not a default side effect of analysis.** Every
  reverse-engineering workflow may report candidates. Applying them is authorized only when the
  selected skill's description explicitly promises Ghidra synchronization, the invocation includes
  `--sync-ghidra-labels`, or the user directly requests synchronization. `--no-sync-ghidra-labels`
  or any read-only request disables it. Workers are always read-only; after all readers stop, only
  the root or sole agent may mutate Ghidra, serially.
- **Apply only certain, low-risk metadata.** A function label requires the exact boundary, behavior,
  owner/receiver, and relevant active caller binding. A global/data label requires the exact storage
  boundary and size, verified role, writer/initializer, consuming use, and active data binding. If
  identity or binding is uncertain, keep `FUN_*`/`DAT_*`; an evidence comment may record only the
  verified partial fact and must state the uncertainty. A synthetic memory reference requires proved
  source instruction/table-slot bytes, exact target, operand, reference kind, and confirmed absence
  of an equivalent reference. Never infer metadata from Rust comments, research prose, YRpp,
  neighboring patterns, or an existing Ghidra name, and never remove analyzer references
  automatically. After every authorized mutation: `save_program`, read it back, then continue. If
  write tools are unavailable, report the queue without claiming application. Function creation,
  prototypes, structs, field/type edits, variable renames, and byte patches require separate explicit
  per-task authorization.
- **Never invent** offsets, addresses, vtable slots, fields, enum values, or labels; not verified
  this session → `UNKNOWN`/`UNCHECKED`. Cite the decompile call inline for anything written into a
  doc, and treat your own prior claims as unverified.
- **TS legacy is the most frequent error from decompilation.** Before implementing any behavior
  found in the binary, confirm it is reachable and visible in a normal YR skirmish. Known dormant,
  do NOT implement as default: **fog of war** (`FogOfWar` defaults to `false` — shroud only),
  **subterranean locomotion** (absent from RA2/YR; not the same as low-bridge `TubeClass`
  movement, which *is* active), and most `SpecialFlags`-gated features.

## System Map

`docs/system-map/` is navigation — not parity proof, completion ledger, or work queue, and the
surface is frozen (2026-07-27). Use `loop` and `mechanism` lookups *after* you have a symptom;
never select work from missing fields or unmapped rows. Extend a loop only when verified work
touches that system, updating just the affected nodes/edges, then run
`python -m tools.system_map check --require-sources`. Never bulk-import call adjacency,
bulk-annotate, or hand-edit generated files.

## Change management

- Reduce the request to a compact task contract — player scenario, scope, non-deferrable
  constraints, smallest production validation, residual risk, stop condition — then take the
  simplest robust Rust-native solution.
- Read the relevant code and data before editing; generate multiple hypotheses before a
  non-trivial fix.
- **Debug end-to-end first** — log each pipeline stage (lookup → transform → output) and run once
  before deep-diving any stage statically.
- **Verify the end-to-end result**, not just compilation. When removing or refactoring, trace all
  consumers — the removed system may have masked bugs elsewhere.
- **Read the whole closed loop before fixing one stage** (harvest/return/dock/exit,
  build/place/sell, dock approach/link/deposit/depart); a symptom in one stage often originates in
  another.
- **If a fix makes things worse, stop and reassess** instead of layering more changes.
- No autonomous bulk refactors, renames, or rewrites without explicit approval.
- **A shadow-mode slice flips to authoritative within two sessions or gets reverted.**

## Parallel sessions

- A build failing in files you did NOT modify is another session's work. Don't fix, revert, or
  stash it.
- **Pending re-baselines.** If your change legitimately shifts a committed golden but the tree also
  carries another session's unmerged shifts, do NOT re-baseline — record one line in
  `docs/scans/PENDING_REBASELINES.md` and leave the test red. Whoever later finds the tree clean
  folds all pending entries into one commit citing them.
- Scan findings decay in days; check `git log --grep` and the live code before implementing one.

## Git

- Start development from an up-to-date `main` and commit every change on a short-lived
  `feature/<topic>` branch. Never commit or push directly to `main`; `main` moves only through a
  reviewed PR (or an explicit user-owned GitHub action).
- When this checkout has a sole owner, create or switch the feature branch here. When another
  task owns the checkout, use a separate worktree and feature branch. Continue an existing
  task-owned feature branch rather than creating a second branch for the same work.
- Push a feature branch or open/update its PR only when the user or goal authorizes publication.
  Every PR targets `main`. Delete merged feature branches when safe. Do not create a long-lived
  `dev` branch; use a temporary integration branch only when the user explicitly requests one.
- Gitignored, local-only — never write commit steps for these: `docs/` (except `_config.yml`,
  `index.md`, `system-map/`, `research/`, and `plans/`), `ini/`, `tools/research_index/.cache/`, `.mcp.json`, `.claude/` and
  `.agents/` (except their `skills/` trees, which are tracked — `ghidra-up` alone stays local),
  `todo/`, and root `*.md` other than `README.md`, `CLAUDE.md`, `ENGINE.md`, and `AGENTS.md`
  (`LOCAL.md` stays local).

## Cargo and environment

- Package is `vera20k`; a wrong `-p` exits 101 without running anything. Read and report the
  literal `test result:` line rather than inferring success from completion.
- **Run the cheapest test tier that answers the question.** *Working:* `cargo check -p vera20k`
  plus the touched module only, `cargo test -p vera20k --lib <module_path>::` — always `--lib`, or
  you also link 13 unrelated side binaries. *PR integration/merge certification:* one full
  `cargo test -p vera20k --lib`, run once before the PR is declared ready for `main`, not per
  slice or per commit. *Nightly:* full suite plus
  slow/ignored retail certification; a red nightly is stop-the-line.
- **Never run cargo commands in parallel** from one session; check first with
  `Get-Process cargo,rustc -ErrorAction SilentlyContinue` and wait if another session owns Cargo. A
  full build takes minutes — start it once in the background, then wait on the `test result:` line
  with a single polling loop.
- **Never kill a cargo build mid-compile** — interrupted codegen corrupts
  `target/debug/incremental` and the *next* build fails to link with `unresolved external symbol
  anon.*.llvm.*` in untouched files. Recovery: delete that directory and rebuild.
- **Never run crate-wide `cargo fmt`** — the repo isn't uniformly rustfmt-clean. Format only leaf
  files you edited (`rustfmt --edition 2024 <file>`), never a `mod.rs` (it recurses).
- Coordinate `SNAPSHOT_VERSION` changes and golden re-baselines; one session at a time.
- GPU, linker toolchain, retail-asset location, and other machine-local environment notes
  live in `LOCAL.md` (gitignored, machine-local).
