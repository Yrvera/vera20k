"""Validation for the semantic mechanism-block layer."""

from __future__ import annotations

from pathlib import Path

from .evidence_validation import (
    validate_evidence_tree,
    validate_native_anchors,
    validate_native_edge_evidence,
    validate_observation_commits,
    validate_rust_edge_evidence,
    validate_rust_surfaces,
)
from .jsonio import load_json_strict
from .model import (
    COMMIT_RE,
    EDGE_PLANES,
    MECHANISM_EDGE_ID_RE,
    MECHANISM_ID_RE,
    MECHANISMS_PATH,
    Diagnostic,
    SystemMapError,
    canonical_system_id,
    loop_stage_ids,
)


MECHANISM_SCHEMA_VERSION = 1
MECHANISM_BLOCK_PATTERN = r"^MBLK-[0-9]{3}-[A-Z0-9-]+$"
MECHANISM_EDGE_PATTERN = r"^MBEDGE-[0-9]{4}-[A-Z0-9-]+$"
MECHANISM_EDGE_KINDS = frozenset(
    {
        "requires",
        "ordered_before",
        "handoff_to",
        "emits_to",
        "consumes",
        "gated_by",
        "presents",
        "plays_audio",
    }
)
ACTIVATION_MODES = frozenset(
    {
        "command",
        "event",
        "lifecycle",
        "scheduled",
        "state-transition",
        "presentation",
    }
)
ACTIVATION_STOCK_STATUSES = frozenset(
    {"STOCK_ACTIVE", "MODE_ACTIVE", "CONTENT_CONDITIONAL", "UNCHECKED"}
)
AUTHORITY_ASPECTS = frozenset(
    {
        "command",
        "state",
        "algorithm",
        "lifecycle",
        "ordering",
        "presentation",
        "audio",
        "rng",
        "persistence",
    }
)
SEMANTIC_KINDS = frozenset(
    {
        "ordering",
        "authority",
        "algorithm",
        "lifecycle",
        "rng",
        "timer",
        "same-tick",
        "state-transition",
        "presentation",
        "audio",
    }
)
SEMANTIC_STATUSES = frozenset(
    {"VERIFIED", "CONFIRMED", "INFERRED", "UNCHECKED"}
)
SEMANTIC_BASES = frozenset(
    {"native-binary", "retail-data", "rust-source", "mixed", "unverified"}
)
QUESTION_IMPACTS = frozenset(
    {
        "MILESTONE-BLOCKING",
        "COMPOUNDING",
        "EXACTIFICATION-RESIDUAL",
        "UNKNOWN-RISK",
    }
)

_ROOT_FIELDS = {
    "schema_version",
    "observed_at_commit",
    "id_policy",
    "blocks",
    "edges",
}
_BLOCK_FIELDS = {
    "name",
    "owner",
    "participants",
    "contract",
    "activation",
    "inputs",
    "authority",
    "steps",
    "outputs",
    "critical_semantics",
    "loop_memberships",
    "native_anchors",
    "rust_surfaces",
    "research_query",
    "evidence",
    "open_questions",
}


def load_mechanisms(repo: Path) -> dict:
    """Load the canonical mechanism document with strict JSON semantics."""

    value = load_json_strict(repo / MECHANISMS_PATH)
    if not isinstance(value, dict):
        raise SystemMapError(
            [
                Diagnostic(
                    "error",
                    "MECHANISMS_NOT_OBJECT",
                    "mechanisms root must be an object",
                    path=MECHANISMS_PATH.as_posix(),
                )
            ]
        )
    return value


def validate_mechanisms(
    repo: Path,
    mechanisms: dict,
    known_systems: set[str],
    group_systems: set[str],
    topology: dict,
    diagnostics: list[Diagnostic],
    *,
    require_paths: bool,
) -> None:
    """Validate mechanism identities, contracts, loop coverage, and edge planes."""

    _require_exact_fields(
        mechanisms,
        _ROOT_FIELDS,
        diagnostics,
        record_id="",
        location="$",
    )
    schema_version = mechanisms.get("schema_version")
    if (
        not isinstance(schema_version, int)
        or isinstance(schema_version, bool)
        or schema_version != MECHANISM_SCHEMA_VERSION
    ):
        _error(
            diagnostics,
            "MECHANISM_SCHEMA_VERSION",
            f"mechanisms schema_version must be {MECHANISM_SCHEMA_VERSION}",
            path=MECHANISMS_PATH.as_posix(),
        )
    observed = mechanisms.get("observed_at_commit")
    if not isinstance(observed, str) or not COMMIT_RE.fullmatch(observed):
        _error(
            diagnostics,
            "INVALID_MECHANISM_OBSERVED_COMMIT",
            "mechanisms observed_at_commit must be a Git commit ID",
            path=MECHANISMS_PATH.as_posix(),
        )
    _validate_id_policy(mechanisms.get("id_policy"), diagnostics)

    loops = topology.get("loops", {})
    known_loops = set(loops) if isinstance(loops, dict) else set()
    blocks = mechanisms.get("blocks")
    valid_blocks: set[str] = set()
    block_loops: dict[str, set[str]] = {}
    if not isinstance(blocks, dict):
        _error(
            diagnostics,
            "INVALID_MECHANISM_BLOCKS",
            "mechanism blocks must be an object",
            field="blocks",
        )
    else:
        for block_id, block in sorted(blocks.items()):
            if not MECHANISM_ID_RE.fullmatch(block_id):
                _error(
                    diagnostics,
                    "INVALID_MECHANISM_ID",
                    f"invalid mechanism block ID: {block_id!r}",
                    record_id=block_id,
                )
                continue
            valid_blocks.add(block_id)
            _validate_block(
                repo,
                block_id,
                block,
                known_systems,
                group_systems,
                loops if isinstance(loops, dict) else {},
                diagnostics,
                require_paths=require_paths,
            )
            if isinstance(block, dict):
                block_loops[block_id] = {
                    item.get("loop")
                    for item in block.get("loop_memberships", [])
                    if isinstance(item, dict)
                    and isinstance(item.get("loop"), str)
                }

    valid_edges = _validate_edges(
        repo,
        mechanisms.get("edges"),
        valid_blocks,
        block_loops,
        known_loops,
        diagnostics,
    )
    _validate_requires_cycles(valid_edges, valid_blocks, diagnostics)
    validate_evidence_tree(
        repo,
        mechanisms,
        diagnostics,
        require_paths=require_paths,
        location="$.mechanisms",
    )
    validate_observation_commits(repo, mechanisms, diagnostics)


def _validate_id_policy(value: object, diagnostics: list[Diagnostic]) -> None:
    if not isinstance(value, dict):
        _error(
            diagnostics,
            "INVALID_MECHANISM_ID_POLICY",
            "mechanism id_policy must be an object",
            field="id_policy",
        )
        return
    required = {"block_pattern", "edge_pattern", "rule"}
    _require_exact_fields(
        value,
        required,
        diagnostics,
        record_id="",
        location="id_policy",
    )
    if value.get("block_pattern") != MECHANISM_BLOCK_PATTERN:
        _error(
            diagnostics,
            "INVALID_MECHANISM_BLOCK_PATTERN",
            "id_policy block_pattern does not match the canonical policy",
            field="id_policy.block_pattern",
        )
    if value.get("edge_pattern") != MECHANISM_EDGE_PATTERN:
        _error(
            diagnostics,
            "INVALID_MECHANISM_EDGE_PATTERN",
            "id_policy edge_pattern does not match the canonical policy",
            field="id_policy.edge_pattern",
        )
    if not _nonempty(value.get("rule")):
        _error(
            diagnostics,
            "MISSING_MECHANISM_ID_RULE",
            "id_policy rule must be non-empty",
            field="id_policy.rule",
        )


def _validate_block(
    repo: Path,
    block_id: str,
    block: object,
    known_systems: set[str],
    group_systems: set[str],
    loops: dict,
    diagnostics: list[Diagnostic],
    *,
    require_paths: bool,
) -> None:
    if not isinstance(block, dict):
        _error(
            diagnostics,
            "INVALID_MECHANISM_BLOCK",
            "mechanism block must be an object",
            record_id=block_id,
        )
        return
    _require_exact_fields(
        block,
        _BLOCK_FIELDS,
        diagnostics,
        record_id=block_id,
        location=f"blocks.{block_id}",
    )
    for field in ("name", "contract", "research_query"):
        if not _nonempty(block.get(field)):
            _error(
                diagnostics,
                "EMPTY_MECHANISM_FIELD",
                f"mechanism {field} must be non-empty",
                record_id=block_id,
                field=field,
            )
    research_query = block.get("research_query")
    if isinstance(research_query, str) and len(research_query) > 1_000:
        _error(
            diagnostics,
            "MECHANISM_RESEARCH_QUERY_TOO_LONG",
            "research_query must not exceed 1000 characters",
            record_id=block_id,
            field="research_query",
        )

    owner = block.get("owner")
    participants = _system_list(
        block.get("participants"),
        known_systems,
        diagnostics,
        record_id=block_id,
        field="participants",
        require_nonempty=True,
    )
    _system_ref(
        owner,
        known_systems,
        diagnostics,
        record_id=block_id,
        field="owner",
    )
    if isinstance(owner, str) and owner not in participants:
        _error(
            diagnostics,
            "MECHANISM_OWNER_NOT_PARTICIPANT",
            "mechanism owner must be present in participants",
            record_id=block_id,
            field="owner",
        )
    if isinstance(owner, str) and owner in group_systems:
        _error(
            diagnostics,
            "MECHANISM_GROUP_OWNER",
            "mechanism owner must be an atomic GSI system, not a group node",
            record_id=block_id,
            field="owner",
        )

    referenced: set[str] = set()
    if isinstance(owner, str):
        referenced.add(owner)
    _validate_activation(block_id, block.get("activation"), diagnostics)
    referenced.update(
        _validate_inputs(
            block_id,
            block.get("inputs"),
            known_systems,
            diagnostics,
        )
    )
    referenced.update(
        _validate_authority(
            block_id,
            block.get("authority"),
            known_systems,
            diagnostics,
        )
    )
    referenced.update(
        _validate_steps(
            block_id,
            block.get("steps"),
            known_systems,
            diagnostics,
        )
    )
    referenced.update(
        _validate_outputs(
            block_id,
            block.get("outputs"),
            known_systems,
            diagnostics,
        )
    )
    _validate_semantics(block_id, block.get("critical_semantics"), diagnostics)
    _validate_loop_memberships(
        block_id,
        block.get("loop_memberships"),
        loops,
        participants,
        diagnostics,
    )
    _validate_open_questions(
        block_id, block.get("open_questions"), diagnostics
    )
    if referenced - participants:
        _error(
            diagnostics,
            "MECHANISM_PARTICIPANTS_INCOMPLETE",
            "participants omit referenced systems: "
            + ", ".join(sorted(referenced - participants)),
            record_id=block_id,
            field="participants",
        )

    validate_native_anchors(
        block.get("native_anchors"),
        diagnostics,
        record_id=block_id,
    )
    _validate_native_anchor_shape(
        block_id, block.get("native_anchors"), diagnostics
    )
    _validate_rust_surface_shape(
        repo,
        block_id,
        block.get("rust_surfaces"),
        diagnostics,
        require_paths=require_paths,
    )
    validate_rust_surfaces(
        repo,
        block.get("rust_surfaces"),
        diagnostics,
        record_id=block_id,
        field="rust_surfaces",
        require_paths=require_paths,
        require_observation=True,
    )
    evidence = block.get("evidence")
    if not isinstance(evidence, list) or not evidence:
        _error(
            diagnostics,
            "MISSING_MECHANISM_EVIDENCE",
            "mechanism block requires evidence",
            record_id=block_id,
            field="evidence",
        )
    _validate_evidence_shape(
        evidence,
        diagnostics,
        record_id=block_id,
        field="evidence",
        require_nonempty=True,
    )


def _validate_activation(
    block_id: str, value: object, diagnostics: list[Diagnostic]
) -> None:
    fields = {"mode", "stock_status", "trigger", "stock_fixture", "guards"}
    if not isinstance(value, dict):
        _error(
            diagnostics,
            "INVALID_MECHANISM_ACTIVATION",
            "activation must be an object",
            record_id=block_id,
            field="activation",
        )
        return
    _require_exact_fields(
        value,
        fields,
        diagnostics,
        record_id=block_id,
        location="activation",
    )
    mode = value.get("mode")
    if not isinstance(mode, str) or mode not in ACTIVATION_MODES:
        _error(
            diagnostics,
            "INVALID_ACTIVATION_MODE",
            f"unsupported activation mode {value.get('mode')!r}",
            record_id=block_id,
            field="activation.mode",
        )
    stock_status = value.get("stock_status")
    if (
        not isinstance(stock_status, str)
        or stock_status not in ACTIVATION_STOCK_STATUSES
    ):
        _error(
            diagnostics,
            "INVALID_ACTIVATION_STOCK_STATUS",
            f"unsupported activation stock_status "
            f"{value.get('stock_status')!r}",
            record_id=block_id,
            field="activation.stock_status",
        )
    for field in ("trigger", "stock_fixture"):
        if not _nonempty(value.get(field)):
            _error(
                diagnostics,
                "EMPTY_ACTIVATION_FIELD",
                f"activation {field} must be non-empty",
                record_id=block_id,
                field=f"activation.{field}",
            )
    _string_list(
        value.get("guards"),
        diagnostics,
        record_id=block_id,
        field="activation.guards",
        require_nonempty=False,
    )


def _validate_inputs(
    block_id: str,
    value: object,
    known_systems: set[str],
    diagnostics: list[Diagnostic],
) -> set[str]:
    referenced: set[str] = set()
    if not isinstance(value, list) or not value:
        _error(
            diagnostics,
            "INVALID_MECHANISM_INPUTS",
            "inputs must be a non-empty array",
            record_id=block_id,
            field="inputs",
        )
        return referenced
    allowed = {"name", "detail", "producer_systems", "external_source"}
    names: set[str] = set()
    for index, item in enumerate(value):
        field = f"inputs[{index}]"
        if not isinstance(item, dict):
            _error(
                diagnostics,
                "INVALID_MECHANISM_INPUT",
                "input must be an object",
                record_id=block_id,
                field=field,
            )
            continue
        _require_allowed_fields(
            item,
            allowed,
            {"name", "detail"},
            diagnostics,
            record_id=block_id,
            location=field,
        )
        _require_text_fields(item, ("name", "detail"), block_id, field, diagnostics)
        name = item.get("name")
        if isinstance(name, str) and name.strip():
            if name in names:
                _error(
                    diagnostics,
                    "DUPLICATE_MECHANISM_INPUT_NAME",
                    f"duplicate input name {name!r}",
                    record_id=block_id,
                    field=f"{field}.name",
                )
            names.add(name)
        producers = item.get("producer_systems")
        if producers is not None:
            referenced.update(
                _system_list(
                    producers,
                    known_systems,
                    diagnostics,
                    record_id=block_id,
                    field=f"{field}.producer_systems",
                    require_nonempty=True,
                )
            )
        if producers is None and not _nonempty(item.get("external_source")):
            _error(
                diagnostics,
                "MECHANISM_INPUT_HAS_NO_SOURCE",
                "input requires producer_systems or external_source",
                record_id=block_id,
                field=field,
            )
        elif (
            "external_source" in item
            and not _nonempty(item.get("external_source"))
        ):
            _error(
                diagnostics,
                "EMPTY_MECHANISM_EXTERNAL_SOURCE",
                "external_source must be non-empty when present",
                record_id=block_id,
                field=f"{field}.external_source",
            )
    return referenced


def _validate_authority(
    block_id: str,
    value: object,
    known_systems: set[str],
    diagnostics: list[Diagnostic],
) -> set[str]:
    referenced: set[str] = set()
    if not isinstance(value, list) or not value:
        _error(
            diagnostics,
            "INVALID_MECHANISM_AUTHORITY",
            "authority must be a non-empty array",
            record_id=block_id,
            field="authority",
        )
        return referenced
    fields = {"aspect", "owner", "detail"}
    for index, item in enumerate(value):
        field = f"authority[{index}]"
        if not isinstance(item, dict):
            _error(
                diagnostics,
                "INVALID_AUTHORITY_RECORD",
                "authority record must be an object",
                record_id=block_id,
                field=field,
            )
            continue
        _require_exact_fields(
            item, fields, diagnostics, record_id=block_id, location=field
        )
        aspect = item.get("aspect")
        if not isinstance(aspect, str) or aspect not in AUTHORITY_ASPECTS:
            _error(
                diagnostics,
                "INVALID_AUTHORITY_ASPECT",
                f"unsupported authority aspect {item.get('aspect')!r}",
                record_id=block_id,
                field=f"{field}.aspect",
            )
        owner = item.get("owner")
        _system_ref(
            owner,
            known_systems,
            diagnostics,
            record_id=block_id,
            field=f"{field}.owner",
        )
        if isinstance(owner, str):
            referenced.add(owner)
        if not _nonempty(item.get("detail")):
            _error(
                diagnostics,
                "EMPTY_AUTHORITY_DETAIL",
                "authority detail must be non-empty",
                record_id=block_id,
                field=f"{field}.detail",
            )
    return referenced


def _validate_steps(
    block_id: str,
    value: object,
    known_systems: set[str],
    diagnostics: list[Diagnostic],
) -> set[str]:
    referenced: set[str] = set()
    if not isinstance(value, list) or not value:
        _error(
            diagnostics,
            "INVALID_MECHANISM_STEPS",
            "steps must be a non-empty array",
            record_id=block_id,
            field="steps",
        )
        return referenced
    orders: list[int] = []
    fields = {"order", "system", "action"}
    for index, item in enumerate(value):
        field = f"steps[{index}]"
        if not isinstance(item, dict):
            _error(
                diagnostics,
                "INVALID_MECHANISM_STEP",
                "step must be an object",
                record_id=block_id,
                field=field,
            )
            continue
        _require_exact_fields(
            item, fields, diagnostics, record_id=block_id, location=field
        )
        order = item.get("order")
        if not isinstance(order, int) or isinstance(order, bool) or order < 1:
            _error(
                diagnostics,
                "INVALID_MECHANISM_STEP_ORDER",
                "step order must be a positive integer",
                record_id=block_id,
                field=f"{field}.order",
            )
        else:
            orders.append(order)
        system_id = item.get("system")
        _system_ref(
            system_id,
            known_systems,
            diagnostics,
            record_id=block_id,
            field=f"{field}.system",
        )
        if isinstance(system_id, str):
            referenced.add(system_id)
        if not _nonempty(item.get("action")):
            _error(
                diagnostics,
                "EMPTY_MECHANISM_STEP_ACTION",
                "step action must be non-empty",
                record_id=block_id,
                field=f"{field}.action",
            )
    expected = list(range(1, len(value) + 1))
    if orders != expected:
        _error(
            diagnostics,
            "NONCONTIGUOUS_MECHANISM_STEPS",
            f"step orders must be contiguous and stored as {expected}",
            record_id=block_id,
            field="steps",
        )
    return referenced


def _validate_outputs(
    block_id: str,
    value: object,
    known_systems: set[str],
    diagnostics: list[Diagnostic],
) -> set[str]:
    referenced: set[str] = set()
    if not isinstance(value, list) or not value:
        _error(
            diagnostics,
            "INVALID_MECHANISM_OUTPUTS",
            "outputs must be a non-empty array",
            record_id=block_id,
            field="outputs",
        )
        return referenced
    allowed = {"name", "detail", "consumer_systems", "player_visible"}
    names: set[str] = set()
    for index, item in enumerate(value):
        field = f"outputs[{index}]"
        if not isinstance(item, dict):
            _error(
                diagnostics,
                "INVALID_MECHANISM_OUTPUT",
                "output must be an object",
                record_id=block_id,
                field=field,
            )
            continue
        _require_allowed_fields(
            item,
            allowed,
            {"name", "detail"},
            diagnostics,
            record_id=block_id,
            location=field,
        )
        _require_text_fields(item, ("name", "detail"), block_id, field, diagnostics)
        name = item.get("name")
        if isinstance(name, str) and name.strip():
            if name in names:
                _error(
                    diagnostics,
                    "DUPLICATE_MECHANISM_OUTPUT_NAME",
                    f"duplicate output name {name!r}",
                    record_id=block_id,
                    field=f"{field}.name",
                )
            names.add(name)
        consumers = item.get("consumer_systems")
        if consumers is not None:
            referenced.update(
                _system_list(
                    consumers,
                    known_systems,
                    diagnostics,
                    record_id=block_id,
                    field=f"{field}.consumer_systems",
                    require_nonempty=True,
                )
            )
        visible = item.get("player_visible")
        if visible is not None and not isinstance(visible, bool):
            _error(
                diagnostics,
                "INVALID_PLAYER_VISIBLE_FLAG",
                "player_visible must be boolean",
                record_id=block_id,
                field=f"{field}.player_visible",
            )
        if consumers is None and visible is not True:
            _error(
                diagnostics,
                "MECHANISM_OUTPUT_HAS_NO_CONSUMER",
                "output requires consumer_systems or player_visible=true",
                record_id=block_id,
                field=field,
            )
    return referenced


def _validate_semantics(
    block_id: str, value: object, diagnostics: list[Diagnostic]
) -> None:
    if not isinstance(value, list) or not value:
        _error(
            diagnostics,
            "INVALID_CRITICAL_SEMANTICS",
            "critical_semantics must be a non-empty array",
            record_id=block_id,
            field="critical_semantics",
        )
        return
    allowed = {"kind", "status", "basis", "detail", "evidence"}
    for index, item in enumerate(value):
        field = f"critical_semantics[{index}]"
        if not isinstance(item, dict):
            _error(
                diagnostics,
                "INVALID_CRITICAL_SEMANTIC",
                "critical semantic must be an object",
                record_id=block_id,
                field=field,
            )
            continue
        _require_allowed_fields(
            item,
            allowed,
            {"kind", "status", "basis", "detail"},
            diagnostics,
            record_id=block_id,
            location=field,
        )
        semantic_kind = item.get("kind")
        if (
            not isinstance(semantic_kind, str)
            or semantic_kind not in SEMANTIC_KINDS
        ):
            _error(
                diagnostics,
                "INVALID_SEMANTIC_KIND",
                f"unsupported semantic kind {item.get('kind')!r}",
                record_id=block_id,
                field=f"{field}.kind",
            )
        status = item.get("status")
        if not isinstance(status, str) or status not in SEMANTIC_STATUSES:
            _error(
                diagnostics,
                "INVALID_SEMANTIC_STATUS",
                f"unsupported semantic status {status!r}",
                record_id=block_id,
                field=f"{field}.status",
            )
        basis = item.get("basis")
        if not isinstance(basis, str) or basis not in SEMANTIC_BASES:
            _error(
                diagnostics,
                "INVALID_SEMANTIC_BASIS",
                f"unsupported semantic basis {basis!r}",
                record_id=block_id,
                field=f"{field}.basis",
            )
        if status == "VERIFIED" and (
            not isinstance(basis, str)
            or basis not in {"native-binary", "retail-data", "mixed"}
        ):
            _error(
                diagnostics,
                "UNSUPPORTED_VERIFIED_SEMANTIC_BASIS",
                "VERIFIED semantics require native-binary, retail-data, or mixed evidence",
                record_id=block_id,
                field=f"{field}.basis",
            )
        if status == "CONFIRMED" and basis != "rust-source":
            _error(
                diagnostics,
                "INVALID_CONFIRMED_SEMANTIC_BASIS",
                "CONFIRMED semantics require basis=rust-source and do not "
                "claim native parity",
                record_id=block_id,
                field=f"{field}.basis",
            )
        if status == "UNCHECKED" and basis != "unverified":
            _error(
                diagnostics,
                "UNCHECKED_SEMANTIC_BASIS",
                "UNCHECKED semantics must use basis=unverified",
                record_id=block_id,
                field=f"{field}.basis",
            )
        if not _nonempty(item.get("detail")):
            _error(
                diagnostics,
                "EMPTY_SEMANTIC_DETAIL",
                "critical semantic detail must be non-empty",
                record_id=block_id,
                field=f"{field}.detail",
            )
        evidence = item.get("evidence")
        if evidence is not None and (
            not isinstance(evidence, list) or not evidence
        ):
            _error(
                diagnostics,
                "INVALID_SEMANTIC_EVIDENCE",
                "critical semantic evidence must be a non-empty array when present",
                record_id=block_id,
                field=f"{field}.evidence",
            )
        if evidence is not None:
            _validate_evidence_shape(
                evidence,
                diagnostics,
                record_id=block_id,
                field=f"{field}.evidence",
                require_nonempty=True,
            )
        if isinstance(status, str) and status in {
            "VERIFIED",
            "CONFIRMED",
            "INFERRED",
        } and (
            not isinstance(evidence, list) or not evidence
        ):
            _error(
                diagnostics,
                "SEMANTIC_STATUS_WITHOUT_EVIDENCE",
                f"{status} critical semantic requires evidence",
                record_id=block_id,
                field=f"{field}.evidence",
            )
        _validate_semantic_evidence_basis(
            block_id, field, status, basis, evidence, diagnostics
        )


def _validate_loop_memberships(
    block_id: str,
    value: object,
    loops: dict,
    participants: set[str],
    diagnostics: list[Diagnostic],
) -> None:
    if not isinstance(value, list) or not value:
        _error(
            diagnostics,
            "INVALID_LOOP_MEMBERSHIPS",
            "loop_memberships must be a non-empty array",
            record_id=block_id,
            field="loop_memberships",
        )
        return
    seen_loops: set[str] = set()
    for index, membership in enumerate(value):
        field = f"loop_memberships[{index}]"
        if not isinstance(membership, dict):
            _error(
                diagnostics,
                "INVALID_LOOP_MEMBERSHIP",
                "loop membership must be an object",
                record_id=block_id,
                field=field,
            )
            continue
        _require_exact_fields(
            membership,
            {"loop", "stage_orders"},
            diagnostics,
            record_id=block_id,
            location=field,
        )
        loop_id = membership.get("loop")
        if not isinstance(loop_id, str) or loop_id not in loops:
            _error(
                diagnostics,
                "UNKNOWN_MECHANISM_LOOP",
                f"mechanism references unknown loop {loop_id!r}",
                record_id=block_id,
                field=f"{field}.loop",
            )
            continue
        if loop_id in seen_loops:
            _error(
                diagnostics,
                "DUPLICATE_MECHANISM_LOOP",
                f"mechanism repeats loop membership {loop_id}",
                record_id=block_id,
                field=field,
            )
        seen_loops.add(loop_id)
        orders = membership.get("stage_orders")
        if not isinstance(orders, list) or not orders:
            _error(
                diagnostics,
                "INVALID_MECHANISM_STAGE_ORDERS",
                "stage_orders must be a non-empty array",
                record_id=block_id,
                field=f"{field}.stage_orders",
            )
            continue
        if (
            any(
                not isinstance(order, int)
                or isinstance(order, bool)
                or order < 1
                for order in orders
            )
            or orders != sorted(set(orders))
        ):
            _error(
                diagnostics,
                "INVALID_MECHANISM_STAGE_ORDERS",
                "stage_orders must be unique positive integers in ascending order",
                record_id=block_id,
                field=f"{field}.stage_orders",
            )
            continue
        stage_ids, stage_order_values = loop_stage_ids(loops[loop_id])
        normalized_orders = [
            order if order is not None else index + 1
            for index, order in enumerate(stage_order_values)
        ]
        stage_by_order = dict(zip(normalized_orders, stage_ids, strict=True))
        for order in orders:
            if order not in stage_by_order:
                _error(
                    diagnostics,
                    "UNKNOWN_MECHANISM_LOOP_STAGE",
                    f"{loop_id} has no stage {order}",
                    record_id=block_id,
                    field=f"{field}.stage_orders",
                )
                continue
            stage_system = stage_by_order[order]
            if stage_system not in participants:
                _error(
                    diagnostics,
                    "MECHANISM_STAGE_NOT_PARTICIPANT",
                    f"{loop_id} stage {order} system {stage_system} is not a participant",
                    record_id=block_id,
                    field=f"{field}.stage_orders",
                )


def _validate_native_anchor_shape(
    block_id: str, value: object, diagnostics: list[Diagnostic]
) -> None:
    if not isinstance(value, list):
        return
    for index, anchor in enumerate(value):
        if isinstance(anchor, str):
            if not anchor.strip():
                _error(
                    diagnostics,
                    "EMPTY_NATIVE_ANCHOR",
                    "native anchor string must be non-empty",
                    record_id=block_id,
                    field=f"native_anchors[{index}]",
                )
            continue
        if not isinstance(anchor, dict):
            continue
        _require_exact_fields(
            anchor,
            {"symbol", "address", "evidence"},
            diagnostics,
            record_id=block_id,
            location=f"native_anchors[{index}]",
        )
        _validate_evidence_shape(
            [anchor.get("evidence")],
            diagnostics,
            record_id=block_id,
            field=f"native_anchors[{index}].evidence",
            require_nonempty=True,
        )


def _validate_rust_surface_shape(
    repo: Path,
    block_id: str,
    value: object,
    diagnostics: list[Diagnostic],
    *,
    require_paths: bool,
) -> None:
    if not isinstance(value, list):
        return
    for index, surface in enumerate(value):
        field = f"rust_surfaces[{index}]"
        if not isinstance(surface, dict):
            continue
        _require_allowed_fields(
            surface,
            {"path", "symbol", "coverage", "observed_at_commit"},
            {"path", "coverage", "observed_at_commit"},
            diagnostics,
            record_id=block_id,
            location=field,
        )
        symbol = surface.get("symbol")
        if symbol is not None and not _nonempty(symbol):
            _error(
                diagnostics,
                "EMPTY_RUST_SURFACE_SYMBOL",
                "Rust surface symbol must be non-empty when present",
                record_id=block_id,
                field=f"{field}.symbol",
            )
        path = surface.get("path")
        if (
            require_paths
            and isinstance(path, str)
            and isinstance(symbol, str)
            and symbol.strip()
        ):
            normalized = path.strip().replace("\\", "/")
            absolute = repo / normalized
            if absolute.is_file():
                symbol_leaf = symbol.rsplit("::", 1)[-1]
                source = absolute.read_text(encoding="utf-8", errors="replace")
                if symbol_leaf not in source:
                    _error(
                        diagnostics,
                        "MISSING_RUST_SURFACE_SYMBOL",
                        f"mapped Rust symbol {symbol!r} is absent from {normalized}",
                        record_id=block_id,
                        field=f"{field}.symbol",
                    )


def _validate_evidence_shape(
    value: object,
    diagnostics: list[Diagnostic],
    *,
    record_id: str,
    field: str,
    require_nonempty: bool,
) -> None:
    """Enforce the mechanism schema's citation-array shape at runtime."""

    if not isinstance(value, list) or (require_nonempty and not value):
        _error(
            diagnostics,
            "INVALID_MECHANISM_EVIDENCE_SHAPE",
            "evidence must be a non-empty citation array"
            if require_nonempty
            else "evidence must be a citation array",
            record_id=record_id,
            field=field,
        )
        return
    for index, citation in enumerate(value):
        citation_field = f"{field}[{index}]"
        if isinstance(citation, str):
            if not citation.strip():
                _error(
                    diagnostics,
                    "INVALID_MECHANISM_CITATION",
                    "citation string must be non-empty",
                    record_id=record_id,
                    field=citation_field,
                )
            continue
        if not isinstance(citation, dict):
            _error(
                diagnostics,
                "INVALID_MECHANISM_CITATION",
                "citation must be a string or object",
                record_id=record_id,
                field=citation_field,
            )
            continue
        _require_allowed_fields(
            citation,
            {"path", "line", "start_line", "end_line"},
            {"path"},
            diagnostics,
            record_id=record_id,
            location=citation_field,
        )
        if not _nonempty(citation.get("path")):
            _error(
                diagnostics,
                "INVALID_MECHANISM_CITATION_PATH",
                "citation path must be non-empty",
                record_id=record_id,
                field=f"{citation_field}.path",
            )
        for line_field in ("line", "start_line", "end_line"):
            line = citation.get(line_field)
            if line is not None and (
                not isinstance(line, int) or isinstance(line, bool) or line < 1
            ):
                _error(
                    diagnostics,
                    "INVALID_MECHANISM_CITATION_LINE",
                    f"{line_field} must be a positive integer",
                    record_id=record_id,
                    field=f"{citation_field}.{line_field}",
                )


def _validate_semantic_evidence_basis(
    block_id: str,
    field: str,
    status: object,
    basis: object,
    evidence: object,
    diagnostics: list[Diagnostic],
) -> None:
    if status == "UNCHECKED" or not isinstance(evidence, list):
        return
    paths = {_evidence_path(item) for item in evidence}
    paths.discard(None)
    has_research = any(path.startswith("docs/research/") for path in paths)
    has_retail = any(path.startswith(("ini/", "art/")) for path in paths)
    has_rust = any(path.startswith(("src/", "tests/")) for path in paths)
    if basis == "native-binary" and not has_research:
        _error(
            diagnostics,
            "NATIVE_SEMANTIC_WITHOUT_RESEARCH",
            "native-binary semantic evidence must cite docs/research",
            record_id=block_id,
            field=f"{field}.evidence",
        )
    elif basis == "retail-data" and not (has_retail or has_research):
        _error(
            diagnostics,
            "RETAIL_SEMANTIC_WITHOUT_RETAIL_EVIDENCE",
            "retail-data semantic evidence must cite INI/art data or "
            "docs/research",
            record_id=block_id,
            field=f"{field}.evidence",
        )
    elif basis == "rust-source" and not has_rust:
        _error(
            diagnostics,
            "RUST_SEMANTIC_WITHOUT_RUST_EVIDENCE",
            "rust-source semantic evidence must cite src/tests",
            record_id=block_id,
            field=f"{field}.evidence",
        )
    elif basis == "mixed" and not (
        has_research and (has_retail or has_rust)
    ):
        _error(
            diagnostics,
            "MIXED_SEMANTIC_WITHOUT_MIXED_EVIDENCE",
            "mixed semantic evidence must include docs/research plus INI/art "
            "or Rust evidence",
            record_id=block_id,
            field=f"{field}.evidence",
        )


def _evidence_path(value: object) -> str | None:
    candidate = value.get("path") if isinstance(value, dict) else value
    if not isinstance(candidate, str):
        return None
    normalized = candidate.strip().replace("\\", "/")
    prefix, separator, suffix = normalized.rpartition(":")
    if separator and suffix.replace("-", "").isdigit():
        normalized = prefix
    return normalized


def _validate_open_questions(
    block_id: str, value: object, diagnostics: list[Diagnostic]
) -> None:
    if not isinstance(value, list):
        _error(
            diagnostics,
            "INVALID_OPEN_QUESTIONS",
            "open_questions must be an array",
            record_id=block_id,
            field="open_questions",
        )
        return
    fields = {"detail", "impact"}
    for index, item in enumerate(value):
        field = f"open_questions[{index}]"
        if not isinstance(item, dict):
            _error(
                diagnostics,
                "INVALID_OPEN_QUESTION",
                "open question must be an object",
                record_id=block_id,
                field=field,
            )
            continue
        _require_exact_fields(
            item, fields, diagnostics, record_id=block_id, location=field
        )
        if not _nonempty(item.get("detail")):
            _error(
                diagnostics,
                "EMPTY_OPEN_QUESTION",
                "open question detail must be non-empty",
                record_id=block_id,
                field=f"{field}.detail",
            )
        impact = item.get("impact")
        if not isinstance(impact, str) or impact not in QUESTION_IMPACTS:
            _error(
                diagnostics,
                "INVALID_OPEN_QUESTION_IMPACT",
                f"unsupported open-question impact {item.get('impact')!r}",
                record_id=block_id,
                field=f"{field}.impact",
            )


def _validate_edges(
    repo: Path,
    value: object,
    known_blocks: set[str],
    block_loops: dict[str, set[str]],
    known_loops: set[str],
    diagnostics: list[Diagnostic],
) -> list[dict]:
    if not isinstance(value, list):
        _error(
            diagnostics,
            "INVALID_MECHANISM_EDGES",
            "mechanism edges must be an array",
            field="edges",
        )
        return []
    allowed = {
        "id",
        "plane",
        "kind",
        "from",
        "to",
        "detail",
        "context",
        "loop",
        "observed_at_commit",
        "evidence",
    }
    required = {"id", "plane", "kind", "from", "to", "detail"}
    seen: set[str] = set()
    seen_semantics: set[tuple[object, ...]] = set()
    valid: list[dict] = []
    for index, edge in enumerate(value):
        field = f"edges[{index}]"
        if not isinstance(edge, dict):
            _error(
                diagnostics,
                "INVALID_MECHANISM_EDGE",
                "mechanism edge must be an object",
                field=field,
            )
            continue
        _require_allowed_fields(
            edge,
            allowed,
            required,
            diagnostics,
            record_id=str(edge.get("id", "")),
            location=field,
        )
        edge_id = edge.get("id")
        if not isinstance(edge_id, str) or not MECHANISM_EDGE_ID_RE.fullmatch(
            edge_id
        ):
            _error(
                diagnostics,
                "INVALID_MECHANISM_EDGE_ID",
                f"invalid mechanism edge ID: {edge_id!r}",
                field=f"{field}.id",
            )
            continue
        duplicate = edge_id in seen
        if duplicate:
            _error(
                diagnostics,
                "DUPLICATE_MECHANISM_EDGE_ID",
                f"duplicate mechanism edge ID {edge_id}",
                record_id=edge_id,
            )
        seen.add(edge_id)
        plane = edge.get("plane")
        kind = edge.get("kind")
        source = edge.get("from")
        target = edge.get("to")
        plane_valid = isinstance(plane, str) and plane in EDGE_PLANES
        kind_valid = (
            isinstance(kind, str) and kind in MECHANISM_EDGE_KINDS
        )
        source_valid = isinstance(source, str) and source in known_blocks
        target_valid = isinstance(target, str) and target in known_blocks
        if not plane_valid:
            _error(
                diagnostics,
                "INVALID_MECHANISM_EDGE_PLANE",
                f"unsupported edge plane {plane!r}",
                record_id=edge_id,
                field=f"{field}.plane",
            )
        if not kind_valid:
            _error(
                diagnostics,
                "INVALID_MECHANISM_EDGE_KIND",
                f"unsupported edge kind {kind!r}",
                record_id=edge_id,
                field=f"{field}.kind",
            )
        for endpoint_name, endpoint in (("from", source), ("to", target)):
            if not isinstance(endpoint, str) or endpoint not in known_blocks:
                _error(
                    diagnostics,
                    "UNKNOWN_MECHANISM_EDGE_ENDPOINT",
                    f"edge {endpoint_name} references unknown block {endpoint!r}",
                    record_id=edge_id,
                    field=f"{field}.{endpoint_name}",
                )
        if source_valid and target_valid and source == target:
            _error(
                diagnostics,
                "SELF_MECHANISM_EDGE",
                "mechanism edge cannot target its source",
                record_id=edge_id,
            )
        if not _nonempty(edge.get("detail")):
            _error(
                diagnostics,
                "EMPTY_MECHANISM_EDGE_DETAIL",
                "mechanism edge detail must be non-empty",
                record_id=edge_id,
                field=f"{field}.detail",
            )
        loop_id = edge.get("loop")
        loop_valid = loop_id is None or (
            isinstance(loop_id, str) and loop_id in known_loops
        )
        if not loop_valid:
            _error(
                diagnostics,
                "UNKNOWN_MECHANISM_EDGE_LOOP",
                f"mechanism edge references unknown loop {loop_id!r}",
                record_id=edge_id,
                field=f"{field}.loop",
            )
        if (
            isinstance(loop_id, str)
            and loop_id in known_loops
            and (
                loop_id not in block_loops.get(str(source), set())
                or loop_id not in block_loops.get(str(target), set())
            )
        ):
            _error(
                diagnostics,
                "MECHANISM_EDGE_OUTSIDE_LOOP",
                "a loop-scoped mechanism edge requires both endpoints to "
                f"belong to {loop_id}",
                record_id=edge_id,
                field=f"{field}.loop",
            )
        if kind == "ordered_before" and not _nonempty(edge.get("context")):
            _error(
                diagnostics,
                "MECHANISM_ORDER_EDGE_WITHOUT_CONTEXT",
                "ordered_before edge requires a non-empty context",
                record_id=edge_id,
                field=f"{field}.context",
            )
        elif "context" in edge and not _nonempty(edge.get("context")):
            _error(
                diagnostics,
                "INVALID_MECHANISM_EDGE_CONTEXT",
                "edge context must be a non-empty string when present",
                record_id=edge_id,
                field=f"{field}.context",
            )
        if plane == "native":
            validate_native_edge_evidence(
                edge, diagnostics, record_id=edge_id
            )
        elif plane == "rust":
            validate_rust_edge_evidence(
                repo, edge, diagnostics, record_id=edge_id
            )
        evidence = edge.get("evidence")
        if evidence is not None:
            _validate_evidence_shape(
                evidence,
                diagnostics,
                record_id=edge_id,
                field=f"{field}.evidence",
                require_nonempty=True,
            )
        observed = edge.get("observed_at_commit")
        if observed is not None and (
            not isinstance(observed, str) or not COMMIT_RE.fullmatch(observed)
        ):
            _error(
                diagnostics,
                "INVALID_MECHANISM_EDGE_COMMIT",
                "edge observed_at_commit must be a Git commit ID",
                record_id=edge_id,
                field=f"{field}.observed_at_commit",
            )
        if (
            plane_valid
            and kind_valid
            and source_valid
            and target_valid
            and loop_valid
        ):
            semantic_key = (plane, kind, source, target, loop_id)
            if semantic_key in seen_semantics:
                _error(
                    diagnostics,
                    "DUPLICATE_MECHANISM_EDGE",
                    "duplicate semantic mechanism edge",
                    record_id=edge_id,
                    field=field,
                )
            else:
                seen_semantics.add(semantic_key)
        if (
            duplicate
            or not plane_valid
            or not kind_valid
            or not source_valid
            or not target_valid
            or not loop_valid
        ):
            continue
        valid.append(edge)
    return valid


def _validate_requires_cycles(
    edges: list[dict],
    known_blocks: set[str],
    diagnostics: list[Diagnostic],
) -> None:
    for plane in sorted(EDGE_PLANES):
        adjacency = {block_id: set() for block_id in known_blocks}
        for edge in edges:
            if edge.get("kind") == "requires" and edge.get("plane") == plane:
                adjacency[edge["from"]].add(edge["to"])
        visiting: set[str] = set()
        visited: set[str] = set()

        def visit(node: str, trail: list[str]) -> None:
            if node in visiting:
                start = trail.index(node)
                cycle = trail[start:] + [node]
                _error(
                    diagnostics,
                    "MECHANISM_REQUIRES_CYCLE",
                    f"{plane} mechanism requires cycle: "
                    + " -> ".join(cycle),
                    record_id=node,
                    field="edges",
                )
                return
            if node in visited:
                return
            visiting.add(node)
            trail.append(node)
            for target in sorted(adjacency[node]):
                visit(target, trail)
            trail.pop()
            visiting.remove(node)
            visited.add(node)

        for block_id in sorted(known_blocks):
            visit(block_id, [])


def _system_ref(
    value: object,
    known_systems: set[str],
    diagnostics: list[Diagnostic],
    *,
    record_id: str,
    field: str,
) -> None:
    if canonical_system_id(value) is None:
        _error(
            diagnostics,
            "INVALID_MECHANISM_SYSTEM_REFERENCE",
            f"not a canonical GSI ID: {value!r}",
            record_id=record_id,
            field=field,
        )
    elif value not in known_systems:
        _error(
            diagnostics,
            "UNKNOWN_MECHANISM_SYSTEM_REFERENCE",
            f"system is absent from registry: {value}",
            record_id=record_id,
            field=field,
        )


def _system_list(
    value: object,
    known_systems: set[str],
    diagnostics: list[Diagnostic],
    *,
    record_id: str,
    field: str,
    require_nonempty: bool,
) -> set[str]:
    if not isinstance(value, list) or (require_nonempty and not value):
        _error(
            diagnostics,
            "INVALID_MECHANISM_SYSTEM_LIST",
            "system references must be a"
            + (" non-empty" if require_nonempty else "")
            + " array",
            record_id=record_id,
            field=field,
        )
        return set()
    result: set[str] = set()
    for index, system_id in enumerate(value):
        _system_ref(
            system_id,
            known_systems,
            diagnostics,
            record_id=record_id,
            field=f"{field}[{index}]",
        )
        if isinstance(system_id, str):
            if system_id in result:
                _error(
                    diagnostics,
                    "DUPLICATE_MECHANISM_SYSTEM_REFERENCE",
                    f"duplicate system reference {system_id}",
                    record_id=record_id,
                    field=field,
                )
            result.add(system_id)
    return result


def _string_list(
    value: object,
    diagnostics: list[Diagnostic],
    *,
    record_id: str,
    field: str,
    require_nonempty: bool,
) -> None:
    if not isinstance(value, list) or (require_nonempty and not value):
        _error(
            diagnostics,
            "INVALID_MECHANISM_STRING_LIST",
            f"{field} must be an array",
            record_id=record_id,
            field=field,
        )
        return
    strings = [item for item in value if isinstance(item, str) and item.strip()]
    if len(strings) != len(value):
        _error(
            diagnostics,
            "EMPTY_MECHANISM_STRING",
            f"{field} contains a non-string or blank value",
            record_id=record_id,
            field=field,
        )
    if len(strings) != len(set(strings)):
        _error(
            diagnostics,
            "DUPLICATE_MECHANISM_STRING",
            f"{field} contains a duplicate value",
            record_id=record_id,
            field=field,
        )


def _require_text_fields(
    value: dict,
    fields: tuple[str, ...],
    block_id: str,
    location: str,
    diagnostics: list[Diagnostic],
) -> None:
    for field in fields:
        if not _nonempty(value.get(field)):
            _error(
                diagnostics,
                "EMPTY_MECHANISM_TEXT",
                f"{field} must be non-empty",
                record_id=block_id,
                field=f"{location}.{field}",
            )


def _require_exact_fields(
    value: dict,
    expected: set[str],
    diagnostics: list[Diagnostic],
    *,
    record_id: str,
    location: str,
) -> None:
    _require_allowed_fields(
        value,
        expected,
        expected,
        diagnostics,
        record_id=record_id,
        location=location,
    )


def _require_allowed_fields(
    value: dict,
    allowed: set[str],
    required: set[str],
    diagnostics: list[Diagnostic],
    *,
    record_id: str,
    location: str,
) -> None:
    for field in sorted(required - set(value)):
        _error(
            diagnostics,
            "MISSING_MECHANISM_FIELD",
            f"{location} lacks required field {field}",
            record_id=record_id,
            field=f"{location}.{field}",
        )
    for field in sorted(set(value) - allowed):
        _error(
            diagnostics,
            "UNKNOWN_MECHANISM_FIELD",
            f"{location} has unsupported field {field}",
            record_id=record_id,
            field=f"{location}.{field}",
        )


def _nonempty(value: object) -> bool:
    return isinstance(value, str) and bool(value.strip())


def _error(
    diagnostics: list[Diagnostic],
    code: str,
    message: str,
    *,
    record_id: str = "",
    field: str = "",
    path: str = "",
) -> None:
    diagnostics.append(
        Diagnostic(
            "error",
            code,
            message,
            record_id=record_id,
            field=field,
            path=path,
        )
    )
