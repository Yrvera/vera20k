# The Convergence Strategy — how to work and think so vera20k reaches 100%

2026-07-05. Companion to `2026-07-05-project-completion-strategy.md` (the WHAT/backlog); this doc
is the HOW/THINK. Pressure-tested by three adversarial lanes (convergence holes / feasibility /
evidence check); their corrections are folded in and marked where they changed the claim.

---

## 1. The diagnosis (evidence-corrected)

The precise failure is **verification granularity**, not effort:

- **Scan-based verification at SYSTEM granularity asymptotes.** Miner: 187 docs, 10 prior scan
  docs, and the July scan still found 119 confirmed gaps; self-assessed ~70% parity (n=1,
  self-assessed — but only 3 of 173 observable surfaces have EVER been binary-scanned, and the
  projection is 550–1,450 residual gaps). Scans sample; absence of findings proves nothing.
- **Executable-gated verification at SLICE granularity converged every time it was tried.**
  The PerTickUpdate spine spec was adversarially confirmed bit-for-bit against live disassembly;
  S4b's RNG draw was PROVEN neutral by disabling the hash fold; S4a's multi-file flip landed with
  0 regressions across 3,879 tests. When "done" meant "an executable check passes," done stayed done.
- The current gates are **self-referential**: golden hashes, QUICKPLAY smoke, determinism replays
  all test vera20k against its own prior output. They ratchet regressions; they certify nothing
  about gamemd. The only gamemd-side gate in 41 sessions was the user's eyeballs — and the manual
  side-by-side gates have been repeatedly deferred and never run.
- The pipeline binary → prose doc → plan → Rust loses fidelity at every hop; stale prose was the
  measured #1 waste category. Prose can be the *scaffolding*; it cannot be the *certificate*.

**The mental shift: stop treating parity as a search problem ("go find differences") and treat it
as a proof problem with a finite obligation set ("here is everything that must be shown equivalent;
show it, check it in, never re-litigate it").**

## 2. The five principles (as amended by the adversaries)

### P1 — Make 100% a countable object: the Ledger (machine-derived, never hand-stamped)
Enumerate what must be equivalent:
- **Code:** the YR-skirmish-reachable function set. Static xref closure alone is NOT computable
  for VC++6 (vtables, CLSID locomotor factory, opcode/trigger pointer tables, dialog procs) — the
  closure must be **hybrid**: static xrefs + systematic vtable/RTTI slot harvest + one dynamic
  coverage trace of a real instrumented skirmish. Any function hit at runtime but absent from the
  ledger is automatically a closure bug. Rows: N/A-DEAD (with reachability evidence) or
  DECODED → CONTRACTED → IMPLEMENTED → VERIFIED.
- **Data:** every (file, section, key) the binary actually queries — captured by hooking
  `INIClass::Get*` / CSF / mix lookups at runtime (static call-site xrefs miss sprintf'd keys,
  list-driven loops, theater fallbacks), plus opcodes, mission states, warhead flags, EVA/sound
  events, hardcoded tables.
- **Anti-rot rule (non-negotiable):** hand-maintained trackers were the #1 waste — the ledger is
  only allowed to exist as **derived state**: closure recomputed from Ghidra by script, VERIFIED
  = a named passing test exists in the repo, IMPLEMENTED = symbol-mapped code exists. If a human
  (or agent) has to remember to update it, it is already rotting.

### P2 — Executable oracles; the eyeball becomes a spot-check, not the gate
Six oracle classes, each with a purity/feasibility boundary (adversary-corrected):
1. **Function vectors** (Ghidra emulation → cargo tests). Purity-classify every row:
   pure-narrow-domain functions (facing math: 256 inputs) get **exhaustive** vectors — that IS
   proof; pure-wide get boundary+random vectors marked *evidence, not proof*; stateful functions
   CANNOT be certified by I/O pairs — they get VERIFIED only from trace/match oracles.
   Vectors are always machine-derived (hand-computed goldens measurably failed).
2. **Trace fixtures** — the existing golden-hash harness, but populated with **gamemd-derived**
   values instead of Rust-vs-prior-Rust.
3. **Match-level differential** — real gamemd (clean retail install confirmed, no anti-debug,
   DDrawCompat present) vs vera20k on the same scenario, per-tick state/RNG digests diffed.
   Corrections: pin gamemd's clock-seeded RNG (debugger write or patch); run gamemd **twice** and
   accept only self-agreeing ticks (retail YR has run-to-run nondeterminism — self-disagreeing
   ticks become explicit "reference undefined" rows); capture via **injected logger DLL** at frame
   boundary, not breakpoint-per-frame (perturbation + throughput; Syringe/Ares ecosystem is strong
   prior art on this exact binary). This is a **serial** instrument — at ~70% parity it diverges
   near tick 1 and you fix one divergence at a time; it is the per-slice truth source and the
   endgame certifier, not a broad drift detector today.
4. **Pixel goldens** — frame captures of deterministic scenes, vera20k quantized to RGB565 to
   match gamemd's verified output format.
5. **Cadence contracts** (new — timing lives *between* the snapshots of oracles 1–4): measured
   ticks-per-wall-second per game speed, scroll px/sec, cameo-flash and radar-blip periods,
   tooltip delay — extracted once from gamemd, asserted as numeric cargo tests against vera20k's
   pacing. Without this, a 5%-off game feel stays green on every other oracle.
6. **Input contracts** — synthetic Win32-message replay (scripted WM_* stream → expected command
   stream) run against both gamemd's dispatcher (via the logger DLL) and vera20k's input layer;
   edge-scroll geometry, drag thresholds, double-click windows live here, not in sim state.
   (Audio timing currently has NO oracle — flagged open; nearest patch is logger-DLL event
   timestamps + cadence contracts on cue scheduling.)

### P3 — Contracts over docs (reconciled with the docs culture)
`docs/research/` remains the knowledge base and the skills keep feeding on it. What changes:
a research doc is **scaffolding**; only an executable artifact (vector set, fixture, contract
test) closes a ledger row. Every RE session's definition-of-done gains one line: *what test now
exists that did not before?* A session that produces only prose has produced scaffolding, not
progress. (This is "systematize what already worked" — the drive_track byte-equality gate,
acceptance tests, /implementation-contract are this pattern under other names.)

### P4 — Ledger-driven selection, user-decided
Targets get picked (by you) from the ledger dashboard — largest unverified reachable region,
highest player-frequency unverified rows — instead of from memory/vibes/most-recent-doc. Agents
present ledger deltas as options; you choose. Standing CLAUDE.md rule unchanged: the agent
surfaces, the user decides.

### P5 — Ratchet (with the collision rules baked in)
Every VERIFIED row's test runs nightly; the number only goes up. Existing constraints absorbed:
one golden/SNAPSHOT-bumper session at a time; machine-derived baselines only; a red nightly is
a stop-the-line event, not a note.

## 3. What this changes about AI-tool usage

| Was | Becomes |
|---|---|
| Swarms hunt disparities open-endedly | Swarms do **mass enumeration + mass vector generation** (vtable harvest, xref closure, INI-hook log processing, emulation batches) — mechanical, parallel, checkable |
| RE session output = a doc | RE session output = a doc **+ the tests it authorizes** |
| Parity claimed in prose ("matches gamemd") | Parity claimed by pointing at a named passing test |
| User eyeball = the gate | User eyeball = acceptance spot-check on top of green oracles |
| Ghidra used statically only | Debugger/emulator/injection become first-class (with infra-tax mitigations: short attach windows, incremental capture, /ghidra-up-style preflight for the port-8099 debugger server) |

## 4. First steps — three validation spikes, then the instrument build

The oracles are the strategy's load-bearing wall and **none are proven in this environment yet**
(emulate_function used once, timed out; debugger server on port 8099 never launched; no capture
tooling on the gamemd side). Validate cheapest-first:

1. **Emulation spike (~1 h, foreground):** Ghidra up → emulate one already-ported leaf function
   (scenario RNG step) on 5 boundary inputs → compare to the Rust counterpart. 5/5 = oracle 1
   green for leaf functions.
2. **Attach spike (~half day):** launch the Ghidra debugger server; gamemd to main menu
   (windowed/borderless DDrawCompat config first); attach; read one known global; detach cleanly.
   Proves the attach/read chain with zero harness investment.
3. **Pixel spike (~2 h):** one deterministic scene captured from both engines, vera20k quantized
   to RGB565, diffed. Measures format + alignment error in one shot.
4. Then the **logger DLL** (the keystone: coverage tracer for P1 + INI-read hook for the data
   ledger + per-tick digests for oracle 3 + input/audio timestamps for oracles 5/6) — a
   from-scratch build (Rust cdylib + injector), no project prior art but overwhelming external
   prior art on this exact binary. Design it as ONE instrument serving all four jobs.
5. The **scripted-input problem** (driving gamemd reproducibly — YR has no native replay) is the
   long pole of oracle 3; it gets its own design spike, decoupled so it never blocks oracles 1/2/4/5.

The roadmap doc's phases stay valid as the WHAT; this doc governs HOW each phase's work is
considered done. The two meet in the ledger: roadmap phases = big regions of ledger rows.

## 5. The one-sentence version

**Enumerate everything gamemd can do (by machine, not by hand), give every row an executable
equivalence check (matched to what that row's purity actually allows), let real gamemd — pinned,
logged, and run twice — be the source of golden truth instead of prose or eyeballs, pick work off
the unverified rows, and ratchet nightly so the verified count only moves up. 100% stops being a
feeling and becomes a number that runs every night.**
