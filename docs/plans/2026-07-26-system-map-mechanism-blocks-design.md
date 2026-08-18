# System Map Mechanism Blocks Design

## Goal

Add a small, evidence-grounded mechanism graph between canonical GSI systems
and ordered player-visible loops so an implementation agent can see authority,
state flow, sequencing, and consumers without treating Ghidra's code graph or
the research corpus as an execution plan.

The user approved this design direction on 2026-07-26.

## Architecture Context

System Map v2 currently provides three useful levels:

- the 336-row canonical GSI registry;
- typed native/Rust/oracle/routing relationships between GSI systems;
- 12 ordered stock player-visible loops.

The research index independently retrieves documents, evidence anchors, and
Rust touchpoints. `research_navigate` is a façade over those two truth domains;
it does not merge their stores.

Ghidra already exposes binary topology: calls, control flow, data references,
receivers, and addresses inside `gamemd.exe`. That topology is indispensable
evidence, but it does not identify the clean Rust owner, connect a native
function to current Rust surfaces, express a complete player-visible loop, or
distinguish a load-bearing handoff from incidental call adjacency.

Existing traces already use prose mechanism blocks. In particular,
`docs/research/traces/2026-07-25-loop-012-power-outage-recovery-trace.md`
decomposes the outage/recovery journey into sell mission authority, refund,
power reassessment, radar gating, and presentation handoffs. This is a grounded
pilot for turning the concept into validated navigation data.

The mechanism layer belongs inside the System Map tooling boundary but in its
own versioned canonical source. It references research/Ghidra evidence and Rust
surfaces; it does not copy the Ghidra graph into JSON and does not become part
of the research SQLite database.

## Impact Analysis

### Added surfaces

- `system_map/mechanisms.v1.json`: curated mechanism blocks and block-to-block
  relationships.
- `system_map/schemas/mechanisms.v1.schema.json`: structural contract.
- `tools/system_map/mechanism_validation.py`: semantic validation.
- CLI/API/report support for exact block lookup, system/loop membership,
  candidate search, freshness, and deterministic rendering.

### Updated integrations

- `research_navigate` gains mechanism candidates and exact
  `mechanism_id` selection while retaining separate research and topology
  result fields.
- System Map and research-index documentation describe the new layer.
- `AGENTS.md` directs future agents to inspect mechanism blocks after selecting
  a system and loop, and to add blocks only while tracing real production
  journeys.

### Non-scope

- No Rust gameplay, simulation, rendering, INI, asset, Ghidra, or Oracle
  mutation.
- No vector database or research-index schema migration.
- No automatic extraction of mechanism blocks from prose or decompiler output.
- No attempt to populate every system or all 12 loops.
- No parity percentage, completion status, work ownership, or task ledger.

The main migration risk is additive report shape. Existing System Map fields
and topology v2 remain unchanged; mechanism data is added as a separately
versioned input and additive report sections.

## Chosen Approach

Use a separate `mechanisms.v1.json` source loaded and validated by System Map.

Mechanism IDs use `MBLK-NNN-SLUG`; relationship IDs use
`MBEDGE-NNNN-SLUG`. A block records:

- its canonical GSI owner and participating systems;
- stock activation and trigger;
- concise behavior contract;
- named inputs and producers;
- authority for state, lifecycle, ordering, RNG, persistence, presentation,
  audio, or commands;
- ordered internal steps;
- named outputs and consumers;
- critical timing/ordering/lifecycle semantics with honest evidence status;
- exact loop-stage memberships;
- native anchors, current Rust surfaces, research navigation query, evidence,
  and bounded open questions.

Block relationships retain the existing native/Rust/oracle/routing planes.
Native relationships require research evidence. Rust relationships require an
observation commit and Rust evidence. Routing relationships remain navigation
only. Only causal `requires` relationships must be acyclic.

The first canonical data contains seven blocks refined from the power-outage
trace: sell mission authority, refund/teardown commit, house-power
reassessment, powered-radar gating, low-power notification, sidebar power-bar
presentation, and radar/minimap presentation. The three consumer routes stay
separate so native notification order does not silently become a claim about
renderer-relative sidebar, radar, or EVA order. This proves the schema against
one real cross-system loop without inferring new binary facts. Absence of a
block remains `UNMAPPED`, not evidence that a mechanism is unimportant or
missing.

## Player-Experience Detail Ledger

- `MILESTONE-BLOCKING`: A block must preserve the ordered handoffs that make a
  complete ordinary-skirmish loop work. An unordered bag of relevant systems
  would recreate the original integration problem.
  [source: `system_map/README.md`; approved System Map v2 design]
- `MILESTONE-BLOCKING`: Ghidra labels and call adjacency must not become
  semantic ownership automatically. Native anchors remain cited evidence
  hints, while the block contract is curated from verified function bodies,
  callers, state flow, and active-YR reachability.
  [source: `AGENTS.md`, Reverse Engineering Rules]
- `COMPOUNDING`: Native, Rust, Oracle, and routing block relationships must
  remain separate. Otherwise a verified native handoff could falsely imply
  current Rust implements it.
  [source: `system_map/README.md`, Relationship Planes]
- `COMPOUNDING`: Every input names producers and every output names consumers.
  Missing either side would let an agent locally implement a block without
  reconnecting it to the production loop.
- `COMPOUNDING`: Authority and critical semantics must remain explicit for
  lifecycle, ordering, RNG, same-tick effects, persistence, presentation, and
  audio where they matter. Unknowns remain `UNCHECKED`; the schema must not
  force invented values.
- `COMPOUNDING`: Rust surfaces carry observation commits and live Git
  freshness. An unchanged representative mapping remains `UNRESOLVED`, not
  parity proof.
  [source: `tools/system_map/freshness.py`]
- `COMPOUNDING`: Exact `MBLK-*` lookup must fail on an unknown identifier.
  Natural-language block matches are candidates only.
- `EXACTIFICATION-RESIDUAL`: The bootstrap covers one loop rather than the full
  engine. Trigger: an agent selects an unmodeled loop. Effect: it must use the
  system/loop views and current research, then add only the blocks proven by
  that trace. Frequency: expected during incremental adoption. Downstream risk
  is bounded because absence is reported as unmapped, not low priority.
- `UNKNOWN-RISK`: The first block granularity may be slightly too coarse or
  fine for another loop. The pilot therefore keeps the schema additive and
  avoids a closed mechanism-kind taxonomy. Revise granularity from real loop
  use rather than bulk modeling.

## Design

### Components

1. **Canonical mechanism source**
   - Independent schema version and input hash.
   - Object-keyed blocks for deterministic exact lookup.
   - Typed block edges with explicit relationship planes.

2. **Mechanism validator**
   - Enforces IDs, root/record shape, canonical GSI references, non-group
     owners, exact loop and stage references, contiguous ordered steps,
     producer/consumer membership, evidence citations, native anchors, Rust
     surfaces, relationship semantics, and causal-cycle rejection.
   - Reuses existing evidence and Rust-surface validators rather than creating
     a second path contract.

3. **Report and freshness integration**
   - Adds normalized block views, mechanism relationships, input provenance,
     block freshness, system memberships, and per-loop ordered block summaries.
   - Does not convert block freshness into parity or completion.

4. **Public API and CLI**
   - `require_mechanism` performs exact lookup.
   - Candidate search ranks systems, loops, and mechanisms independently.
   - `mechanism MBLK-*` prints one block with incoming/outgoing relationships.
   - Existing `show` and `loop` views name related blocks.

5. **Research navigator integration**
   - Adds optional `mechanism_id` and exact-ID query recognition.
   - Returns `selected_mechanism` and bounded mechanism candidates under the
     existing `system_map` field.
   - Uses the selected block's explicit research query and native anchors when
     the request is only an exact `MBLK-*` identifier, and reports that
     substitution transparently.

### Interfaces / Contracts

```text
python -m tools.system_map check --require-sources
python -m tools.system_map mechanism MBLK-001-SELL-MISSION-AUTHORITY
python -m tools.system_map loop LOOP-012-POWER-OUTAGE-RECOVERY
python tools/research_index/navigate.py MBLK-001-SELL-MISSION-AUTHORITY
```

Canonical data is accepted only when the ordinary System Map check succeeds.
Natural-language mechanism matches are navigation candidates, never ownership
or parity claims.

### Data Flow

```text
verified research / Ghidra anchors ─┐
current Rust surfaces ──────────────┼─ curated mechanism block
canonical GSI systems + loop stages ┘
                                      │
                                      ├─ validated block graph
                                      ├─ System Map report/CLI
                                      └─ research_navigate routing context
```

The research index continues to own document retrieval. System Map continues
to own execution topology. Mechanism blocks are the semantic bridge inside the
topology domain.

### Error Handling

Malformed or dangling blocks fail with deterministic diagnostics. Verified
constraints without evidence fail. Unknown evidence remains allowed only when
explicitly marked `UNCHECKED`. Unknown exact IDs are input errors. Missing or
stale Rust surfaces appear through freshness diagnostics and never silently
upgrade a block.

### Testing Strategy

- Minimal valid and malformed block fixtures.
- Unknown GSI/loop/stage references and group-owner rejection.
- Ordered-step and causal-cycle validation.
- Native/Rust relationship evidence requirements.
- Exact API/CLI lookup and candidate-only ranking.
- System and loop membership views.
- Block Git-freshness behavior.
- Navigator exact selection, transparent research seed use, ambiguity, zero
  matches, JSON/text bounds, and MCP argument plumbing.
- Live `check --require-sources`, deterministic render/check, focused System
  Map tests, focused research navigator tests, and real CLI smoke checks.
- No Cargo.

## Architectural Decisions

- Follow the existing standard-library Python, strict JSON, validated report,
  thin CLI, and public read-only API patterns.
- Preserve topology v2 and research SQLite as independent existing contracts.
- Keep mechanism blocks compact and reusable rather than copying whole traces.
- Add blocks only from a real active loop with citations.
- Treat Ghidra topology as evidence input, not a substitute for semantic
  ownership or production-loop mapping.

No gameplay or determinism debt is introduced.

## Alternatives Considered

### Embed blocks directly in `topology.v2.json`

This would make one file atomic, but it would enlarge an already active schema,
increase merge conflicts with ordinary topology maintenance, and make the
macro-system and mechanism layers harder to distinguish. Rejected.

### Automatically extract blocks from Ghidra or research prose

This could populate the graph rapidly, but call adjacency does not prove
authority and stale or contradictory documents would become false structure.
Rejected.

### Add a vector database

Embeddings could improve fuzzy retrieval, but they cannot establish causal
dependencies, ordered handoffs, authority, activation, or truth status. The
current research index already supplies bounded retrieval. Deferred as optional
retrieval polish rather than the mechanism backbone.
