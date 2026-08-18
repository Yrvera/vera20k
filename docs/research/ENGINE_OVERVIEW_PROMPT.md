# Next-session prompt — engine-wide "where do we stand / what's next"

Paste the block below to start the new session.

---

GOAL: we are reimplementing gamemd.exe (Red Alert 2: Yuri's Revenge) natively in Rust. Step back
from individual features. I want a clear OVERVIEW of where the engine actually stands and the SINGLE
most important next thing to build. I suspect some foundational systems are still missing or shaky.
This is read-only research — no code changes. Keep it high-level and simple, not a giant micro-list.

DO THIS:

1. Map what EXISTS. Read `src/lib.rs`, `src/sim/mod.rs`, and each module's `//!` header, plus
   `World::advance_tick` (the sim tick pipeline). That is the authoritative layout of what's built.

2. Map what gamemd NEEDS for a working YR skirmish: map + INI load, rendering, movement/locomotion,
   pathfinding, combat/damage, production/build/place, power, radar + shroud/fog, harvesting/economy,
   superweapons, garrison/occupancy, audio, UI/sidebar, multiplayer lockstep/determinism, save/load.
   (Skip AI and tunnels/subterranean — out of scope for now.)

3. For each system rate maturity — MISSING / STUB / PARTIAL / NEAR-PARITY — with ONE line of evidence
   (the Rust file + whether tests/research back it). Use the research-index MCP
   (`research_map`, `research_search`) to see what's documented vs implemented. Map the current
   code directly rather than relying on a generic broad gap inventory.

4. Find the FOUNDATIONAL gaps: systems that are missing/shaky AND that many other systems sit on top of
   (e.g. if lockstep determinism, or the coordinate/height substrate, or pathfinding, or save/load is
   weak, everything above it inherits the weakness). Rank by how much fixing each one unblocks.

5. OUTPUT: a SHORT "state of the engine" overview — a maturity table + the 3–5 biggest foundational
   gaps — and recommend the ONE system to focus on next, with why. Save it to
   `docs/research/ENGINE_STATE_OVERVIEW.md`. Keep it an overview, not a 47-item list.

Judge every "is this foundation good enough" against the scale target (20k units, 30 players). If you
fan this out as a workflow (one reader per system is a good fit), throttle to small waves (≤3–4
concurrent) and run any cargo as a separate foreground pass — never inside the workflow.

CONTEXT: the bridge subsystem was just deep-scanned and partially fixed; its full parity contract is in
`docs/research/bridges/BRIDGE_PARITY_IMPLEMENTATION_CONTRACT.md` if you want a worked example of how one
system maps to gamemd — but the point of THIS session is the engine-wide picture and the next big rock,
not bridges.
