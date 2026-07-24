#!/usr/bin/env python3
"""Validate the bounded Mission authority writer/caller rollout.

The checker is intentionally read-only. It strips Rust comments and literals,
ignores code owned by test-only items, and compares production occurrences
against explicit path allowlists. Run it from any directory:

    python tools/check_mission_authority_census.py
"""

from __future__ import annotations

import posixpath
import re
import sys
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable, Mapping


ROOT = Path(__file__).resolve().parents[1]
SRC_ROOT = ROOT / "src"

STATE_PATH = "src/sim/mission/state.rs"
COMPATIBILITY_PATH = "src/sim/mission/compatibility.rs"
VERB_PATH = "src/sim/mission/verb.rs"
LEAF_PATH = "src/sim/mission/leaf.rs"
AUTHORITY_PATH = "src/sim/mission/authority.rs"

EXPECTED_COMPATIBILITY: dict[str, frozenset[str]] = {
    "legacy_full_retask": frozenset(
        {
            "src/sim/mission/retask.rs",
            "src/sim/docking/bunker_link.rs",
        }
    ),
    "legacy_current_only_retask": frozenset(
        {
            "src/sim/mission/retask.rs",
        }
    ),
    "legacy_unit_host_projection": frozenset(
        {
            "src/sim/world/techno_ai.rs",
        }
    ),
    "legacy_tick_tail_projection": frozenset(
        {
            "src/sim/world/mod.rs",
        }
    ),
}

EXPECTED_COMPATIBILITY_CALLSITES: dict[str, Mapping[str, tuple[str, ...]]] = {
    "legacy_full_retask": {
        "src/sim/mission/retask.rs": ("assign_mission_with_teardown",),
        "src/sim/docking/bunker_link.rs": (
            "install_bunker_link",
            "release_normal",
        ),
    },
    "legacy_current_only_retask": {
        "src/sim/mission/retask.rs": ("assign_mission_keep_fields",),
    },
    "legacy_unit_host_projection": {
        "src/sim/world/techno_ai.rs": ("unit_techno_bracket",),
    },
    "legacy_tick_tail_projection": {
        "src/sim/world/mod.rs": ("refresh_mission_shadow_except",),
    },
}

EXPECTED_COMPATIBILITY_COUNTS: dict[str, Mapping[str, int]] = {
    token: {
        path: len(scopes)
        for path, scopes in paths.items()
    }
    for token, paths in EXPECTED_COMPATIBILITY_CALLSITES.items()
}

EXACT_AUTHORITY_TOKENS = frozenset(
    {
        "mission_assign_exact",
        "mission_queue_exact",
        "mission_commence_exact",
        "mission_override_exact",
        "mission_restore_exact",
        "mission_refinery_completion_exact",
        "mission_jumpjet_move_to_completion_exact",
        "mission_try_consume_building_ready_exact",
    }
)

STATE_TRANSITION_ALLOWLIST: dict[str, frozenset[str]] = {
    "legacy_full_retask": frozenset(
        {
            STATE_PATH,
            COMPATIBILITY_PATH,
            *EXPECTED_COMPATIBILITY["legacy_full_retask"],
        }
    ),
    "legacy_current_only_retask": frozenset(
        {
            STATE_PATH,
            COMPATIBILITY_PATH,
            *EXPECTED_COMPATIBILITY["legacy_current_only_retask"],
        }
    ),
    "legacy_unit_host_projection": frozenset(
        {
            COMPATIBILITY_PATH,
            *EXPECTED_COMPATIBILITY["legacy_unit_host_projection"],
        }
    ),
    "legacy_tick_tail_projection": frozenset(
        {
            COMPATIBILITY_PATH,
            *EXPECTED_COMPATIBILITY["legacy_tick_tail_projection"],
        }
    ),
    "legacy_projection": frozenset({STATE_PATH, COMPATIBILITY_PATH}),
    "assign_transition": frozenset({STATE_PATH, VERB_PATH}),
    "write_queue_and_clear_b8": frozenset({STATE_PATH, VERB_PATH}),
    "promote_queue": frozenset({STATE_PATH, VERB_PATH}),
    "override_transition": frozenset({STATE_PATH, VERB_PATH}),
    "restore_transition": frozenset({STATE_PATH, VERB_PATH}),
    "increment_ai_counter": frozenset({STATE_PATH, AUTHORITY_PATH}),
    "write_dispatch_epilogue": frozenset({STATE_PATH, AUTHORITY_PATH}),
    "set_movement_bypass_after_verified_queue": frozenset(
        {STATE_PATH, AUTHORITY_PATH}
    ),
}

VERB_WRITER_ALLOWLIST: dict[str, frozenset[str]] = {
    "assign_base": frozenset({VERB_PATH, AUTHORITY_PATH}),
    "queue_base": frozenset({VERB_PATH, AUTHORITY_PATH}),
    "commence_base": frozenset({VERB_PATH, AUTHORITY_PATH}),
    "override_base": frozenset({VERB_PATH, AUTHORITY_PATH}),
    "restore_base": frozenset({VERB_PATH, AUTHORITY_PATH}),
}

LEAF_WRITER_ALLOWLIST: dict[str, frozenset[str]] = {
    "set_unit_deploy_begin_active": frozenset({LEAF_PATH, AUTHORITY_PATH}),
    "set_unit_deploy_reverse_active": frozenset({LEAF_PATH, AUTHORITY_PATH}),
    "set_unit_tracker_byte_18": frozenset({LEAF_PATH, AUTHORITY_PATH}),
    "set_unit_tracker_byte_19": frozenset({LEAF_PATH, AUTHORITY_PATH}),
    "set_infantry_firing_sequence": frozenset({LEAF_PATH, AUTHORITY_PATH}),
    "set_infantry_doing_verified": frozenset({LEAF_PATH, AUTHORITY_PATH}),
    "set_aircraft_transition_ready": frozenset({LEAF_PATH, AUTHORITY_PATH}),
    "clear_aircraft_action_for_commence": frozenset({LEAF_PATH, AUTHORITY_PATH}),
    "set_building_ready_latch": frozenset({LEAF_PATH, AUTHORITY_PATH}),
}

WRITER_ALLOWLIST: dict[str, frozenset[str]] = {
    **STATE_TRANSITION_ALLOWLIST,
    **VERB_WRITER_ALLOWLIST,
    **LEAF_WRITER_ALLOWLIST,
}

WRITER_DEFINITION_FILES: dict[str, frozenset[str]] = {
    STATE_PATH: frozenset(
        {
            "legacy_full_retask",
            "legacy_current_only_retask",
            "legacy_projection",
            "assign_transition",
            "write_queue_and_clear_b8",
            "promote_queue",
            "override_transition",
            "restore_transition",
            "increment_ai_counter",
            "write_dispatch_epilogue",
            "set_movement_bypass_after_verified_queue",
        }
    ),
    COMPATIBILITY_PATH: frozenset(EXPECTED_COMPATIBILITY),
    VERB_PATH: frozenset(VERB_WRITER_ALLOWLIST),
    LEAF_PATH: frozenset(LEAF_WRITER_ALLOWLIST),
}

WRITER_DEFINITION_ALLOWLIST: dict[str, frozenset[str]] = {
    token: frozenset(
        path
        for path, tokens in WRITER_DEFINITION_FILES.items()
        if token in tokens
    )
    for token in WRITER_ALLOWLIST
}

WRITER_CALLER_ALLOWLIST: dict[str, frozenset[tuple[str, str]]] = {
    "legacy_full_retask": frozenset(
        {
            (COMPATIBILITY_PATH, "legacy_full_retask"),
            ("src/sim/mission/retask.rs", "assign_mission_with_teardown"),
            ("src/sim/docking/bunker_link.rs", "install_bunker_link"),
            ("src/sim/docking/bunker_link.rs", "release_normal"),
        }
    ),
    "legacy_current_only_retask": frozenset(
        {
            (COMPATIBILITY_PATH, "legacy_current_only_retask"),
            ("src/sim/mission/retask.rs", "assign_mission_keep_fields"),
        }
    ),
    "legacy_projection": frozenset(
        {
            (COMPATIBILITY_PATH, "legacy_unit_host_projection"),
            (COMPATIBILITY_PATH, "legacy_tick_tail_projection"),
        }
    ),
    "legacy_unit_host_projection": frozenset(
        {("src/sim/world/techno_ai.rs", "unit_techno_bracket")}
    ),
    "legacy_tick_tail_projection": frozenset(
        {("src/sim/world/mod.rs", "refresh_mission_shadow_except")}
    ),
    "assign_transition": frozenset({(VERB_PATH, "assign_base")}),
    "write_queue_and_clear_b8": frozenset({(VERB_PATH, "queue_base")}),
    "promote_queue": frozenset({(VERB_PATH, "commence_base")}),
    "override_transition": frozenset({(VERB_PATH, "override_base")}),
    "restore_transition": frozenset({(VERB_PATH, "restore_base")}),
    "increment_ai_counter": frozenset(),
    "write_dispatch_epilogue": frozenset(),
    "set_movement_bypass_after_verified_queue": frozenset(
        {
            (AUTHORITY_PATH, "mission_refinery_completion_exact"),
            (AUTHORITY_PATH, "mission_jumpjet_move_to_completion_exact"),
            (AUTHORITY_PATH, "validate_jumpjet_second_gate_previews"),
        }
    ),
    "assign_base": frozenset({(AUTHORITY_PATH, "mission_assign_exact")}),
    "queue_base": frozenset(
        {
            (AUTHORITY_PATH, "mission_queue_exact"),
            (AUTHORITY_PATH, "validate_jumpjet_second_gate_previews"),
        }
    ),
    "commence_base": frozenset(
        {
            (AUTHORITY_PATH, "commence_leaf"),
            (AUTHORITY_PATH, "validate_jumpjet_second_gate_previews"),
        }
    ),
    "override_base": frozenset(
        {(AUTHORITY_PATH, "mission_override_exact_with_effects")}
    ),
    "restore_base": frozenset(
        {(AUTHORITY_PATH, "mission_restore_exact_with_effects")}
    ),
    "set_unit_deploy_begin_active": frozenset(),
    "set_unit_deploy_reverse_active": frozenset(),
    "set_unit_tracker_byte_18": frozenset(),
    "set_unit_tracker_byte_19": frozenset(),
    "set_infantry_firing_sequence": frozenset(),
    "set_infantry_doing_verified": frozenset(),
    "set_aircraft_transition_ready": frozenset(),
    "clear_aircraft_action_for_commence": frozenset(
        {(AUTHORITY_PATH, "commence_leaf")}
    ),
    "set_building_ready_latch": frozenset(
        {(AUTHORITY_PATH, "mission_try_consume_building_ready_exact")}
    ),
}

WRITER_TOP_LEVEL_REFERENCE_ALLOWLIST: dict[str, frozenset[str]] = {
    token: frozenset() for token in WRITER_ALLOWLIST
}
WRITER_TOP_LEVEL_REFERENCE_ALLOWLIST["legacy_full_retask"] = frozenset(
    {
        "src/sim/mission/retask.rs",
        "src/sim/docking/bunker_link.rs",
    }
)
WRITER_TOP_LEVEL_REFERENCE_ALLOWLIST[
    "legacy_current_only_retask"
] = frozenset({"src/sim/mission/retask.rs"})
WRITER_TOP_LEVEL_REFERENCE_ALLOWLIST[
    "legacy_unit_host_projection"
] = frozenset({"src/sim/world/techno_ai.rs"})
WRITER_TOP_LEVEL_REFERENCE_ALLOWLIST[
    "legacy_tick_tail_projection"
] = frozenset({"src/sim/world/mod.rs"})

EXACT_INTERNAL_CALLERS: dict[str, frozenset[str]] = {
    token: frozenset() for token in EXACT_AUTHORITY_TOKENS
}
EXACT_INTERNAL_CALLERS["mission_queue_exact"] = frozenset(
    {
        "mission_refinery_completion_exact",
        "mission_jumpjet_move_to_completion_exact",
    }
)

MISSION_FIELDS = (
    "current",
    "suspended",
    "queued",
    "movement_bypass_latch",
    "handler_state",
    "mission_start_frame",
    "ai_counter",
    "dispatch_timer",
)
LEGACY_REDUCED_FIELDS = ("substate", "timer", "tick_counter")
ALL_MISSION_FIELDS = MISSION_FIELDS + LEGACY_REDUCED_FIELDS

STATE_FIELD_WRITERS: dict[str, frozenset[str]] = {
    "current": frozenset(
        {
            "legacy_full_retask",
            "legacy_current_only_retask",
            "legacy_projection",
            "assign_transition",
            "promote_queue",
            "override_transition",
            "restore_transition",
        }
    ),
    "suspended": frozenset(
        {
            "legacy_full_retask",
            "override_transition",
            "restore_transition",
        }
    ),
    "queued": frozenset(
        {
            "legacy_full_retask",
            "assign_transition",
            "write_queue_and_clear_b8",
            "promote_queue",
        }
    ),
    "movement_bypass_latch": frozenset(
        {
            "assign_transition",
            "write_queue_and_clear_b8",
            "promote_queue",
            "override_transition",
            "restore_transition",
            "set_movement_bypass_after_verified_queue",
        }
    ),
    "handler_state": frozenset(
        {
            "legacy_full_retask",
            "legacy_projection",
            "assign_transition",
            "promote_queue",
        }
    ),
    "mission_start_frame": frozenset(
        {
            "assign_transition",
            "promote_queue",
        }
    ),
    "ai_counter": frozenset(
        {
            "legacy_projection",
            "assign_transition",
            "promote_queue",
            "increment_ai_counter",
        }
    ),
    "dispatch_timer": frozenset(
        {
            "legacy_full_retask",
            "assign_transition",
            "promote_queue",
            "write_dispatch_epilogue",
        }
    ),
}

STATE_RAW_WRITERS = frozenset(
    writer
    for writers in STATE_FIELD_WRITERS.values()
    for writer in writers
)

LEAF_FIELD_WRITERS: dict[str, frozenset[str]] = {
    "deploy_begin_active": frozenset({"set_unit_deploy_begin_active"}),
    "deploy_reverse_active": frozenset({"set_unit_deploy_reverse_active"}),
    "tracker_byte_18": frozenset({"set_unit_tracker_byte_18"}),
    "tracker_byte_19": frozenset({"set_unit_tracker_byte_19"}),
    "firing_sequence_latch": frozenset({"set_infantry_firing_sequence"}),
    "doing": frozenset({"set_infantry_doing_verified"}),
    "action_latch": frozenset({"clear_aircraft_action_for_commence"}),
    "transition_ready_latch": frozenset({"set_aircraft_transition_ready"}),
    "airstrike_manager_present": frozenset(),
    "ready_latch": frozenset({"set_building_ready_latch"}),
}

LEAF_RAW_WRITERS = frozenset(
    writer
    for writers in LEAF_FIELD_WRITERS.values()
    for writer in writers
)

# Keep these independent of the maps above so the lexical field census cannot
# silently shrink if a writer is removed.
EXPECTED_STATE_RAW_WRITERS = frozenset(
    {
        "legacy_full_retask",
        "legacy_current_only_retask",
        "legacy_projection",
        "assign_transition",
        "write_queue_and_clear_b8",
        "promote_queue",
        "override_transition",
        "restore_transition",
        "increment_ai_counter",
        "write_dispatch_epilogue",
        "set_movement_bypass_after_verified_queue",
    }
)

LEAF_FIELDS = (
    "deploy_begin_active",
    "deploy_reverse_active",
    "tracker_byte_18",
    "tracker_byte_19",
    "firing_sequence_latch",
    "doing",
    "action_latch",
    "transition_ready_latch",
    "airstrike_manager_present",
    "ready_latch",
)

TEST_ONLY_WRITER_TOKENS = frozenset(
    {
        "apply_test_fixture",
        "for_test_kind",
        "new_at_frame_zero_for_test",
        "set_mission_ready_state_for_test",
        "set_object_is_falling_down_for_test",
        "test_default",
        "unit_raw_for_test",
        "infantry_raw_for_test",
        "aircraft_raw_for_test",
        "building_raw_for_test",
    }
)

SUSPICIOUS_COMMENCE_SCOPE = re.compile(
    r"(?:advance_tick|tick_tail|tail_phase|"
    r"(?:global|all).*(?:entit|object)|"
    r"(?:entit|object).*(?:global|all)|"
    r"(?:tick|update|process|iterate|step).*(?:entities|objects)|"
    r"(?:entities|objects).*(?:tick|update|process|iterate|step)|"
    r"drain.*(?:queue|mission)|(?:queue|mission).*drain)",
    re.IGNORECASE,
)

GLOBAL_ENTITY_ITERATION = re.compile(
    r"(?:"
    r"\b(?:for|while)\b[^{};]{0,320}"
    r"(?:\b(?:entities|objects|logic_order|entity_ids|live_object_order"
    r"|for_each_live_object)\b|\.logic\s*\.)|"
    r"\b(?:entities|objects|logic_order|entity_ids|live_object_order"
    r"|for_each_live_object)\b[^{};]{0,160}"
    r"\.(?:iter|iter_mut|keys|values|values_mut|ids)\s*\("
    r")",
    re.IGNORECASE | re.DOTALL,
)

TEST_ATTRIBUTE = re.compile(
    r"#\s*\[\s*(?:cfg\s*\(\s*test\s*\)|test)\s*\]",
    re.DOTALL,
)
INNER_TEST_ATTRIBUTE = re.compile(
    r"\A\s*#!\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]",
    re.DOTALL,
)
TEST_MODULE_DECL = re.compile(
    r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]"
    r"(?:\s*#\s*\[[^\]]*\])*"
    r"\s*(?:(?:pub(?:\s*\([^)]*\))?|crate)\s+)?"
    r"mod\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*;",
    re.DOTALL,
)
MODULE_DECL = re.compile(
    r"(?P<attrs>(?:#\s*\[[^\]]*\]\s*)*)"
    r"(?:(?:pub(?:\s*\([^)]*\))?|crate)\s+)?"
    r"mod\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*;",
    re.DOTALL,
)
PATH_ATTRIBUTE = re.compile(r"#\s*\[\s*path\s*=\s*\"(?P<path>[^\"]+)\"\s*\]")
FUNCTION_NAME = re.compile(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)")
CHAR_LITERAL = re.compile(
    r"'(?:\\(?:x[0-9A-Fa-f]{2}|u\{[0-9A-Fa-f_]+\}|.)|[^\\'\r\n])'"
)
RAW_STRING_START = re.compile(r"(?:br|r)(?P<hashes>#{0,255})\"")


@dataclass(frozen=True, order=True)
class Location:
    path: str
    line: int
    excerpt: str


@dataclass(frozen=True, order=True)
class Finding:
    code: str
    path: str
    line: int
    message: str


@dataclass
class SourceUnit:
    path: str
    source: str
    code: str
    test_intervals: tuple[tuple[int, int], ...]
    lines: tuple[str, ...]
    declared_test_only: bool = False

    @classmethod
    def build(cls, path: str, source: str) -> "SourceUnit":
        code = sanitize_rust(source)
        intervals = tuple(find_test_intervals(code))
        return cls(
            path,
            source,
            code,
            intervals,
            tuple(source.splitlines()),
            INNER_TEST_ATTRIBUTE.search(code) is not None,
        )

    @property
    def wholly_test_only(self) -> bool:
        return self.declared_test_only

    def is_test_offset(self, offset: int) -> bool:
        if self.wholly_test_only:
            return True
        return any(start <= offset < end for start, end in self.test_intervals)

    def location(self, offset: int) -> Location:
        line = self.code.count("\n", 0, offset) + 1
        excerpt = self.lines[line - 1].strip() if line <= len(self.lines) else ""
        return Location(self.path, line, excerpt)


@dataclass
class CensusResult:
    findings: list[Finding] = field(default_factory=list)
    compatibility_calls: dict[str, list[Location]] = field(default_factory=dict)
    compatibility_call_scopes: dict[
        str, list[tuple[Location, str | None]]
    ] = field(default_factory=dict)
    writer_occurrences: dict[str, list[Location]] = field(default_factory=dict)
    exact_occurrences: dict[str, list[Location]] = field(default_factory=dict)
    state_field_assignments: list[Location] = field(default_factory=list)
    leaf_field_assignments: list[Location] = field(default_factory=list)

    @property
    def ok(self) -> bool:
        return not self.findings


def _blank_range(chars: list[str], source: str, start: int, end: int) -> None:
    for index in range(start, min(end, len(chars))):
        if source[index] not in "\r\n":
            chars[index] = " "


def sanitize_rust(source: str) -> str:
    """Blank comments and literals while preserving offsets and newlines."""

    chars = list(source)
    index = 0
    size = len(source)
    while index < size:
        if source.startswith("//", index):
            end = source.find("\n", index + 2)
            end = size if end == -1 else end
            _blank_range(chars, source, index, end)
            index = end
            continue

        if source.startswith("/*", index):
            depth = 1
            cursor = index + 2
            while cursor < size and depth:
                if source.startswith("/*", cursor):
                    depth += 1
                    cursor += 2
                elif source.startswith("*/", cursor):
                    depth -= 1
                    cursor += 2
                else:
                    cursor += 1
            _blank_range(chars, source, index, cursor)
            index = cursor
            continue

        raw_match = RAW_STRING_START.match(source, index)
        if raw_match and (
            index == 0 or not (source[index - 1].isalnum() or source[index - 1] == "_")
        ):
            hashes = raw_match.group("hashes")
            terminator = '"' + hashes
            end = source.find(terminator, raw_match.end())
            end = size if end == -1 else end + len(terminator)
            _blank_range(chars, source, index, end)
            index = end
            continue

        if source[index] == '"':
            cursor = index + 1
            escaped = False
            while cursor < size:
                char = source[cursor]
                if escaped:
                    escaped = False
                elif char == "\\":
                    escaped = True
                elif char == '"':
                    cursor += 1
                    break
                cursor += 1
            _blank_range(chars, source, index, cursor)
            index = cursor
            continue

        if source[index] == "'":
            char_match = CHAR_LITERAL.match(source, index)
            if char_match:
                _blank_range(chars, source, index, char_match.end())
                index = char_match.end()
                continue

        index += 1

    return "".join(chars)


def _matching_brace(code: str, opening: int) -> int:
    depth = 0
    for index in range(opening, len(code)):
        if code[index] == "{":
            depth += 1
        elif code[index] == "}":
            depth -= 1
            if depth == 0:
                return index + 1
    return len(code)


def _test_target_can_end_with_comma(code: str, start: int) -> bool:
    """Whether the attributed target looks like a field, variant, or parameter."""

    preview = code[start : start + 512]
    boundaries = [
        position
        for delimiter in (":", "=", ",", "{", ";")
        if (position := preview.find(delimiter)) != -1
    ]
    header = preview[: min(boundaries)] if boundaries else preview
    words = set(re.findall(r"\b[A-Za-z_][A-Za-z0-9_]*\b", header))
    item_words = {
        "fn",
        "mod",
        "impl",
        "trait",
        "struct",
        "enum",
        "union",
        "const",
        "static",
        "type",
        "use",
        "extern",
        "macro_rules",
    }
    return words.isdisjoint(item_words)


def _test_item_end(
    code: str, start: int, *, comma_terminates: bool | None = None
) -> int:
    paren_depth = 0
    bracket_depth = 0
    angle_depth = 0
    top_level_equals_seen = False
    if comma_terminates is None:
        comma_terminates = _test_target_can_end_with_comma(code, start)
    for index in range(start, len(code)):
        char = code[index]
        if char == "(":
            paren_depth += 1
        elif char == ")":
            paren_depth = max(0, paren_depth - 1)
        elif char == "[":
            bracket_depth += 1
        elif char == "]":
            bracket_depth = max(0, bracket_depth - 1)
        elif (
            char == "<"
            and comma_terminates
            and bracket_depth == 0
            and not top_level_equals_seen
        ):
            angle_depth += 1
        elif char == ">" and comma_terminates:
            angle_depth = max(0, angle_depth - 1)
        elif (
            char == "="
            and comma_terminates
            and paren_depth == 0
            and bracket_depth == 0
            and angle_depth == 0
        ):
            top_level_equals_seen = True
        elif char == "{" and paren_depth == 0 and bracket_depth == 0:
            return _matching_brace(code, index)
        elif char == ";" and paren_depth == 0 and bracket_depth == 0:
            return index + 1
        elif (
            char == ","
            and comma_terminates
            and paren_depth == 0
            and bracket_depth == 0
            and angle_depth == 0
        ):
            return index + 1
    return len(code)


def find_test_intervals(code: str) -> Iterable[tuple[int, int]]:
    for match in TEST_ATTRIBUTE.finditer(code):
        yield match.start(), _test_item_end(code, match.end())


def _is_definition(code: str, token_offset: int) -> bool:
    prefix = code[max(0, token_offset - 96) : token_offset]
    return re.search(r"\bfn\s*$", prefix) is not None


def _is_invocation(code: str, token_end: int) -> bool:
    return re.match(r"\s*\(", code[token_end:]) is not None


def _production_matches(unit: SourceUnit, pattern: re.Pattern[str]) -> Iterable[re.Match[str]]:
    for match in pattern.finditer(unit.code):
        if not unit.is_test_offset(match.start()):
            yield match


def _function_spans(unit: SourceUnit) -> list[tuple[int, int, str]]:
    spans: list[tuple[int, int, str]] = []
    for match in FUNCTION_NAME.finditer(unit.code):
        if unit.is_test_offset(match.start()):
            continue
        end = _test_item_end(
            unit.code, match.end(), comma_terminates=False
        )
        spans.append((match.start(), end, match.group(1)))
    return spans


def _enclosing_function_span(
    spans: Iterable[tuple[int, int, str]], offset: int
) -> tuple[int, int, str] | None:
    candidates = [
        (end - start, start, end, name)
        for start, end, name in spans
        if start <= offset < end
    ]
    if not candidates:
        return None
    _, start, end, name = min(candidates)
    return start, end, name


def _enclosing_function(
    spans: Iterable[tuple[int, int, str]], offset: int
) -> str | None:
    span = _enclosing_function_span(spans, offset)
    return span[2] if span is not None else None


def _load_repo_sources() -> dict[str, str]:
    sources: dict[str, str] = {}
    for path in sorted(SRC_ROOT.rglob("*.rs")):
        relative = path.relative_to(ROOT).as_posix()
        sources[relative] = path.read_text(encoding="utf-8")
    return sources


def _module_paths_for_match(
    unit: SourceUnit, match: re.Match[str]
) -> tuple[str, ...]:
    source_fragment = unit.source[match.start() : match.end()]
    path_attribute = PATH_ATTRIBUTE.search(source_fragment)
    declaring = Path(unit.path)
    if path_attribute is not None:
        explicit = declaring.parent / path_attribute.group("path")
        return (posixpath.normpath(explicit.as_posix()),)

    if declaring.name in {"lib.rs", "main.rs", "mod.rs"}:
        module_root = declaring.parent
    else:
        module_root = declaring.parent / declaring.stem
    name = match.group("name")
    return (
        (module_root / f"{name}.rs").as_posix(),
        (module_root / name / "mod.rs").as_posix(),
    )


def _mark_declared_test_modules(units: Mapping[str, SourceUnit]) -> None:
    pending: list[str] = []
    for unit in units.values():
        for match in TEST_MODULE_DECL.finditer(unit.code):
            pending.extend(_module_paths_for_match(unit, match))

    visited: set[str] = set()
    while pending:
        path = pending.pop()
        if path in visited:
            continue
        visited.add(path)
        unit = units.get(path)
        if unit is None:
            continue
        unit.declared_test_only = True
        for match in MODULE_DECL.finditer(unit.code):
            pending.extend(_module_paths_for_match(unit, match))


def scan_sources(sources: Mapping[str, str]) -> CensusResult:
    units = {
        path: SourceUnit.build(path, source)
        for path, source in sorted(sources.items())
    }
    _mark_declared_test_modules(units)
    result = CensusResult(
        compatibility_calls={token: [] for token in EXPECTED_COMPATIBILITY},
        compatibility_call_scopes={
            token: [] for token in EXPECTED_COMPATIBILITY
        },
        writer_occurrences={token: [] for token in WRITER_ALLOWLIST},
        exact_occurrences={token: [] for token in EXACT_AUTHORITY_TOKENS},
    )

    implementation_paths = {STATE_PATH, COMPATIBILITY_PATH}
    field_names = "|".join(map(re.escape, ALL_MISSION_FIELDS))
    direct_field = re.compile(
        rf"\bmission\s*\.\s*(?P<field>{field_names})\s*"
        r"(?P<operator>(?:<<|>>|[+\-*/%&|^])?=)(?!=)"
    )
    replace_whole = re.compile(r"\.\s*mission\s*=(?!=)")
    replace_leaf = re.compile(r"\.\s*mission_leaf\s*=(?!=)")
    writer_names = "|".join(
        map(re.escape, sorted(WRITER_ALLOWLIST, key=len, reverse=True))
    )
    writer_pattern = re.compile(rf"\b(?P<token>{writer_names})\b")
    exact_names = "|".join(
        map(re.escape, sorted(EXACT_AUTHORITY_TOKENS, key=len, reverse=True))
    )
    exact_pattern = re.compile(rf"\b(?P<token>{exact_names})\b")
    test_writer_names = "|".join(
        map(re.escape, sorted(TEST_ONLY_WRITER_TOKENS, key=len, reverse=True))
    )
    test_writer_pattern = re.compile(rf"\b(?P<token>{test_writer_names})\s*\(")

    for unit in units.values():
        function_spans: list[tuple[int, int, str]] | None = None
        for match in _production_matches(unit, writer_pattern):
            token = match.group("token")
            location = unit.location(match.start())
            result.writer_occurrences[token].append(location)
            is_definition = _is_definition(unit.code, match.start())
            is_invocation = _is_invocation(unit.code, match.end())
            if unit.path not in WRITER_ALLOWLIST[token]:
                result.findings.append(
                    Finding(
                        "writer-outside-allowlist",
                        location.path,
                        location.line,
                        f"{token}: Mission writer surface is not allowed in this file",
                    )
                )
            elif is_definition:
                if unit.path not in WRITER_DEFINITION_ALLOWLIST[token]:
                    result.findings.append(
                        Finding(
                            "writer-definition-outside-allowlist",
                            location.path,
                            location.line,
                            f"{token}: writer definition is not approved in this file",
                        )
                    )
            else:
                if function_spans is None:
                    function_spans = _function_spans(unit)
                scope = _enclosing_function(function_spans, match.start())
                approved_caller = (
                    scope is not None
                    and (unit.path, scope) in WRITER_CALLER_ALLOWLIST[token]
                )
                if is_invocation and not approved_caller:
                    result.findings.append(
                        Finding(
                            "writer-caller-outside-allowlist",
                            location.path,
                            location.line,
                            f"{token}: unapproved caller scope {scope or '<module>'}",
                        )
                    )
                elif not is_invocation:
                    suffix = unit.code[match.end() : match.end() + 48]
                    if re.match(r"\s+as\b", suffix):
                        result.findings.append(
                            Finding(
                                "writer-alias-reference",
                                location.path,
                                location.line,
                                f"{token}: aliasing an authority writer is forbidden",
                            )
                        )
                    elif scope is not None and not approved_caller:
                        result.findings.append(
                            Finding(
                                "writer-reference-outside-allowlist",
                                location.path,
                                location.line,
                                f"{token}: unapproved writer reference in {scope}",
                            )
                        )
                    elif (
                        scope is None
                        and unit.path
                        not in WRITER_TOP_LEVEL_REFERENCE_ALLOWLIST[token]
                    ):
                        result.findings.append(
                            Finding(
                                "writer-reference-outside-allowlist",
                                location.path,
                                location.line,
                                f"{token}: unapproved top-level writer reference",
                            )
                        )
            if (
                token in EXPECTED_COMPATIBILITY
                and is_invocation
                and not is_definition
                and unit.path not in implementation_paths
            ):
                result.compatibility_calls[token].append(location)
                if function_spans is None:
                    function_spans = _function_spans(unit)
                result.compatibility_call_scopes[token].append(
                    (
                        location,
                        _enclosing_function(function_spans, match.start()),
                    )
                )

        for match in _production_matches(unit, exact_pattern):
            token = match.group("token")
            location = unit.location(match.start())
            result.exact_occurrences[token].append(location)
            if unit.path != AUTHORITY_PATH:
                result.findings.append(
                    Finding(
                        "exact-authority-outside-owner",
                        location.path,
                        location.line,
                        (
                            f"{token}: exact Mission authority must have zero "
                            "external production callers"
                        ),
                    )
                )
                continue

            if _is_definition(unit.code, match.start()):
                continue

            if function_spans is None:
                function_spans = _function_spans(unit)
            scope_span = _enclosing_function_span(
                function_spans, match.start()
            )
            scope = scope_span[2] if scope_span is not None else None
            if scope not in EXACT_INTERNAL_CALLERS[token]:
                result.findings.append(
                    Finding(
                        "exact-internal-caller-outside-allowlist",
                        location.path,
                        location.line,
                        f"{token}: unapproved authority caller {scope or '<module>'}",
                    )
                )

            if token == "mission_commence_exact":
                if function_spans is None:
                    function_spans = _function_spans(unit)
                body_is_global_loop = (
                    scope_span is not None
                    and GLOBAL_ENTITY_ITERATION.search(
                        unit.code[scope_span[0] : scope_span[1]]
                    )
                    is not None
                )
                if scope and (
                    SUSPICIOUS_COMMENCE_SCOPE.search(scope)
                    or body_is_global_loop
                ):
                    result.findings.append(
                        Finding(
                            "generic-commence-drain",
                            location.path,
                            location.line,
                            f"{token}: forbidden generic/tail drain call inside {scope}",
                        )
                    )

        for match in _production_matches(unit, direct_field):
            location = unit.location(match.start())
            result.findings.append(
                Finding(
                    "direct-mission-field-write",
                    location.path,
                    location.line,
                    (
                        "direct assignment to Mission field "
                        f"{match.group('field')}; use a reviewed writer"
                    ),
                )
            )
        for match in _production_matches(unit, replace_whole):
            location = unit.location(match.start())
            result.findings.append(
                Finding(
                    "direct-mission-replacement",
                    location.path,
                    location.line,
                    "whole MissionCom replacement is not an approved production writer",
                )
            )
        for match in _production_matches(unit, replace_leaf):
            location = unit.location(match.start())
            result.findings.append(
                Finding(
                    "direct-mission-leaf-replacement",
                    location.path,
                    location.line,
                    "whole Mission leaf replacement is not an approved production writer",
                )
            )

        for match in _production_matches(unit, test_writer_pattern):
            token = match.group("token")
            location = unit.location(match.start())
            result.findings.append(
                Finding(
                    "test-writer-in-production",
                    location.path,
                    location.line,
                    f"{token}: test-only raw writer used in production",
                )
            )

    for token, expected_paths in sorted(EXPECTED_COMPATIBILITY_CALLSITES.items()):
        actual_by_path: dict[str, list[tuple[Location, str | None]]] = {}
        for location, scope in result.compatibility_call_scopes[token]:
            actual_by_path.setdefault(location.path, []).append(
                (location, scope)
            )
        all_paths = sorted(set(expected_paths) | set(actual_by_path))
        for path in all_paths:
            expected_scopes = Counter(expected_paths.get(path, ()))
            actual_scopes: dict[str | None, list[Location]] = {}
            for location, scope in actual_by_path.get(path, []):
                actual_scopes.setdefault(scope, []).append(location)
            scope_names = sorted(
                set(expected_scopes) | set(actual_scopes),
                key=lambda scope: scope or "",
            )
            for scope in scope_names:
                expected_count = expected_scopes[scope]
                actual_locations = sorted(actual_scopes.get(scope, []))
                if len(actual_locations) < expected_count:
                    result.findings.append(
                        Finding(
                            "compatibility-missing",
                            path,
                            0,
                            (
                                f"{token}: expected {expected_count} call(s) "
                                f"in {scope}, found {len(actual_locations)}"
                            ),
                        )
                    )
                for location in actual_locations[expected_count:]:
                    result.findings.append(
                        Finding(
                            "compatibility-extra",
                            location.path,
                            location.line,
                            f"{token}: unexpected caller scope {scope or '<module>'}",
                        )
                    )

    state_unit = units.get(STATE_PATH)
    if state_unit is not None:
        state_fields = "|".join(map(re.escape, MISSION_FIELDS))
        state_assignment = re.compile(
            rf"\.\s*(?P<field>{state_fields})\s*"
            r"(?:(?:<<|>>|[+\-*/%&|^])?=)(?!=)"
        )
        state_spans = _function_spans(state_unit)
        for match in _production_matches(state_unit, state_assignment):
            location = state_unit.location(match.start())
            result.state_field_assignments.append(location)
            field_name = match.group("field")
            scope = _enclosing_function(state_spans, match.start())
            if scope not in STATE_FIELD_WRITERS[field_name]:
                result.findings.append(
                    Finding(
                        "unapproved-state-raw-writer",
                        location.path,
                        location.line,
                        f"{field_name}: raw write in unapproved scope {scope or '<module>'}",
                    )
                )

    leaf_unit = units.get(LEAF_PATH)
    if leaf_unit is not None:
        leaf_fields = "|".join(map(re.escape, LEAF_FIELDS))
        leaf_assignment = re.compile(
            rf"\.\s*(?P<field>{leaf_fields})\s*"
            r"(?:(?:<<|>>|[+\-*/%&|^])?=)(?!=)"
        )
        leaf_spans = _function_spans(leaf_unit)
        for match in _production_matches(leaf_unit, leaf_assignment):
            location = leaf_unit.location(match.start())
            result.leaf_field_assignments.append(location)
            field_name = match.group("field")
            scope = _enclosing_function(leaf_spans, match.start())
            if scope not in LEAF_FIELD_WRITERS[field_name]:
                result.findings.append(
                    Finding(
                        "unapproved-leaf-raw-writer",
                        location.path,
                        location.line,
                        f"{field_name}: raw write in unapproved scope {scope or '<module>'}",
                    )
                )

    result.findings.sort()
    for locations in result.compatibility_calls.values():
        locations.sort()
    for scoped_locations in result.compatibility_call_scopes.values():
        scoped_locations.sort(key=lambda item: item[0])
    for locations in result.writer_occurrences.values():
        locations.sort()
    for locations in result.exact_occurrences.values():
        locations.sort()
    result.state_field_assignments.sort()
    result.leaf_field_assignments.sort()
    return result


def _baseline_fixture_sources() -> dict[str, str]:
    return {
        "src/sim/mission/retask.rs": (
            "fn assign_mission_with_teardown() { legacy_full_retask(); }\n"
            "fn assign_mission_keep_fields() { legacy_current_only_retask(); }\n"
        ),
        "src/sim/docking/bunker_link.rs": (
            "fn install_bunker_link() { legacy_full_retask(); }\n"
            "fn release_normal() { legacy_full_retask(); }\n"
        ),
        "src/sim/world/techno_ai.rs": (
            "fn unit_techno_bracket() { legacy_unit_host_projection(); }\n"
        ),
        "src/sim/world/mod.rs": (
            "fn refresh_mission_shadow_except() { legacy_tick_tail_projection(); }\n"
        ),
    }


def run_self_tests() -> None:
    if {
        token: frozenset(counts)
        for token, counts in EXPECTED_COMPATIBILITY_COUNTS.items()
    } != EXPECTED_COMPATIBILITY:
        raise AssertionError("compatibility path and count allowlists disagree")
    if STATE_RAW_WRITERS != EXPECTED_STATE_RAW_WRITERS:
        raise AssertionError("state raw-field writer map is incomplete")
    if LEAF_RAW_WRITERS != frozenset(LEAF_WRITER_ALLOWLIST):
        raise AssertionError("leaf raw-field writer map is incomplete")
    if set(WRITER_DEFINITION_ALLOWLIST) != set(WRITER_ALLOWLIST):
        raise AssertionError("writer definition allowlist is incomplete")
    if set(WRITER_CALLER_ALLOWLIST) != set(WRITER_ALLOWLIST):
        raise AssertionError("writer caller allowlist is incomplete")
    if set(WRITER_TOP_LEVEL_REFERENCE_ALLOWLIST) != set(WRITER_ALLOWLIST):
        raise AssertionError("writer top-level reference allowlist is incomplete")
    if set(EXACT_INTERNAL_CALLERS) != set(EXACT_AUTHORITY_TOKENS):
        raise AssertionError("exact internal caller allowlist is incomplete")

    baseline = _baseline_fixture_sources()
    baseline_result = scan_sources(baseline)
    if not baseline_result.ok:
        raise AssertionError(f"baseline fixture failed: {baseline_result.findings}")

    for token in sorted(EXACT_AUTHORITY_TOKENS):
        sources = dict(baseline)
        sources["src/sim/world/rogue.rs"] = f"fn rogue() {{ {token}(); }}\n"
        result = scan_sources(sources)
        if not any(
            finding.code == "exact-authority-outside-owner"
            for finding in result.findings
        ):
            raise AssertionError(f"external {token} was not rejected")

    sources = dict(baseline)
    sources["src/sim/ai.rs"] = "fn extra() { legacy_full_retask(); }\n"
    result = scan_sources(sources)
    if not any(
        finding.code == "compatibility-extra" for finding in result.findings
    ):
        raise AssertionError("extra compatibility caller was not rejected")

    sources = dict(baseline)
    sources["src/sim/mission/retask.rs"] += "fn extra() { legacy_full_retask(); }\n"
    result = scan_sources(sources)
    if not any(
        finding.code == "compatibility-extra" for finding in result.findings
    ):
        raise AssertionError("extra compatibility call in an approved path was not rejected")

    sources = dict(baseline)
    sources["src/sim/world/rogue.rs"] = (
        "use crate::sim::mission::verb::assign_base as hidden_writer;\n"
    )
    result = scan_sources(sources)
    if not any(
        finding.code == "writer-outside-allowlist"
        for finding in result.findings
    ):
        raise AssertionError("aliased Mission writer reference was not rejected")

    for path, source in (
        (
            STATE_PATH,
            (
                "impl MissionCom { fn unauthorized(&mut self) { "
                "self.assign_transition(requested, now); } }\n"
            ),
        ),
        (
            LEAF_PATH,
            (
                "impl MissionLeafState { fn unauthorized(&mut self) { "
                "self.set_unit_tracker_byte_18(1); } }\n"
            ),
        ),
        (
            VERB_PATH,
            "fn unauthorized() { assign_base(state, requested, now); }\n",
        ),
    ):
        sources = dict(baseline)
        sources[path] = source
        result = scan_sources(sources)
        if not any(
            finding.code == "writer-caller-outside-allowlist"
            for finding in result.findings
        ):
            raise AssertionError(f"composite writer wrapper passed in {path}")

    sources = dict(baseline)
    sources[AUTHORITY_PATH] = (
        "fn mission_assign_exact() {}\n"
        "fn unauthorized() { mission_assign_exact(); }\n"
    )
    result = scan_sources(sources)
    if not any(
        finding.code == "exact-internal-caller-outside-allowlist"
        for finding in result.findings
    ):
        raise AssertionError("composite exact-authority wrapper was not rejected")

    sources = dict(baseline)
    sources[AUTHORITY_PATH] = (
        "use crate::sim::mission::verb::assign_base as hidden_writer;\n"
    )
    result = scan_sources(sources)
    if not any(
        finding.code == "writer-alias-reference"
        for finding in result.findings
    ):
        raise AssertionError("writer alias inside an approved file was not rejected")

    sources = dict(baseline)
    sources["src/sim/world/rogue.rs"] = (
        "fn rogue() { entity . mission . current += 1; }\n"
    )
    result = scan_sources(sources)
    if not any(
        finding.code == "direct-mission-field-write"
        for finding in result.findings
    ):
        raise AssertionError("direct Mission field assignment was not rejected")

    sources = dict(baseline)
    sources["src/sim/world/rogue.rs"] = (
        "fn rogue() { entity.mission = MissionCom::at_frame(0); }\n"
    )
    result = scan_sources(sources)
    if not any(
        finding.code == "direct-mission-replacement"
        for finding in result.findings
    ):
        raise AssertionError("whole Mission replacement was not rejected")

    sources = dict(baseline)
    sources["src/sim/world/rogue.rs"] = (
        "fn rogue() { entity.mission_leaf = MissionLeafState::default(); }\n"
    )
    result = scan_sources(sources)
    if not any(
        finding.code == "direct-mission-leaf-replacement"
        for finding in result.findings
    ):
        raise AssertionError("whole Mission leaf replacement was not rejected")

    sources = dict(baseline)
    sources[STATE_PATH] = (
        "impl MissionCom { fn unauthorized(&mut self) { "
        "self.current = MissionId::NONE; } }\n"
    )
    result = scan_sources(sources)
    if not any(
        finding.code == "unapproved-state-raw-writer"
        for finding in result.findings
    ):
        raise AssertionError("unapproved raw MissionCom writer was not rejected")

    sources = dict(baseline)
    sources[LEAF_PATH] = (
        "impl MissionLeafState { fn unauthorized(&mut self) { "
        "self.expect_aircraft_mut().action_latch = 0; } }\n"
    )
    result = scan_sources(sources)
    if not any(
        finding.code == "unapproved-leaf-raw-writer"
        for finding in result.findings
    ):
        raise AssertionError("unapproved raw Mission leaf writer was not rejected")

    sources = dict(baseline)
    sources["src/sim/world/comparison.rs"] = (
        "fn comparison() { let _ = entity.mission.current == MissionId::NONE; }\n"
    )
    result = scan_sources(sources)
    if not result.ok:
        raise AssertionError(f"Mission field comparison was treated as a write: {result.findings}")

    sources = dict(baseline)
    sources["src/sim/world/rogue.rs"] = (
        "fn rogue() { state.apply_test_fixture(fixture); }\n"
    )
    result = scan_sources(sources)
    if not any(
        finding.code == "test-writer-in-production"
        for finding in result.findings
    ):
        raise AssertionError("production use of a test-only writer was not rejected")

    definitions = "\n".join(
        f"pub(crate) fn {token}() {{}}" for token in sorted(EXACT_AUTHORITY_TOKENS)
    )
    test_calls = " ".join(f"{token}();" for token in sorted(EXACT_AUTHORITY_TOKENS))
    sources = dict(baseline)
    sources[AUTHORITY_PATH] = (
        definitions
        + "\n#[cfg(test)] mod tests { fn exact_calls() { "
        + test_calls
        + " } }\n"
    )
    sources["src/sim/world/ignored.rs"] = (
        'const TEXT: &str = "mission_assign_exact(); mission.current = 1;";\n'
        "// mission_queue_exact();\n"
        "#[cfg(test)] mod tests {\n"
        "  fn ignored() {\n"
        "    mission_restore_exact();\n"
        "    entity.mission.current = MissionId::NONE;\n"
        "    legacy_full_retask();\n"
        "  }\n"
        "}\n"
    )
    result = scan_sources(sources)
    if not result.ok:
        raise AssertionError(
            f"authority definitions or test-only bodies were rejected: {result.findings}"
        )

    sources = dict(baseline)
    sources["src/sim/world/mod.rs"] += (
        '\n#[cfg(test)]\n#[path = "dormant_fixture.rs"]\nmod dormant;\n'
    )
    sources["src/sim/world/dormant_fixture.rs"] = (
        "fn ignored() { mission_assign_exact(); state.apply_test_fixture(fixture); }\n"
    )
    result = scan_sources(sources)
    if not result.ok:
        raise AssertionError(
            f"cfg(test)-declared module was treated as production: {result.findings}"
        )

    sources = dict(baseline)
    sources["src/sim/world/inner_cfg_fixture.rs"] = (
        "#![cfg(test)]\n"
        "fn ignored() { mission_assign_exact(); state.apply_test_fixture(fixture); }\n"
    )
    result = scan_sources(sources)
    if not result.ok:
        raise AssertionError(
            f"crate-level cfg(test) was treated as production: {result.findings}"
        )

    for path, source in (
        (
            "src/sim/world/field_mask.rs",
            (
                "struct S { #[cfg(test)] hidden: u8, live: u8 }\n"
                "fn live() { mission_assign_exact(); }\n"
            ),
        ),
        (
            "src/sim/world/rogue_tests.rs",
            "fn live() { mission_assign_exact(); }\n",
        ),
    ):
        sources = dict(baseline)
        sources[path] = source
        result = scan_sources(sources)
        if not any(
            finding.code == "exact-authority-outside-owner"
            for finding in result.findings
        ):
            raise AssertionError(f"production token was hidden by test masking in {path}")

    for scope in (
        "advance_tick",
        "tick_tail_phase",
        "update_all_entities",
        "drain_queued_missions",
    ):
        sources = dict(baseline)
        sources[AUTHORITY_PATH] = (
            definitions
            + f"\nfn {scope}() {{ mission_commence_exact(); }}\n"
        )
        result = scan_sources(sources)
        if not any(
            finding.code == "generic-commence-drain"
            for finding in result.findings
        ):
            raise AssertionError(
                f"exact Commence in forbidden scope {scope} was not rejected"
            )

    sources = dict(baseline)
    sources[AUTHORITY_PATH] = (
        definitions
        + "\nfn run_all() { for id in entities { mission_commence_exact(); } }\n"
    )
    result = scan_sources(sources)
    if not any(
        finding.code == "generic-commence-drain"
        for finding in result.findings
    ):
        raise AssertionError("exact Commence in a global entity loop was not rejected")

    for attribute in (
        "#[cfg(not(test))]",
        '#[cfg(any(test, feature = "live"))]',
    ):
        sources = dict(baseline)
        sources["src/sim/world/rogue.rs"] = (
            f"{attribute}\nfn live() {{ mission_assign_exact(); }}\n"
        )
        result = scan_sources(sources)
        if not any(
            finding.code == "exact-authority-outside-owner"
            for finding in result.findings
        ):
            raise AssertionError(f"{attribute} was incorrectly treated as test-only")


def _format_locations(locations: Iterable[Location]) -> str:
    counts = Counter(location.path for location in locations)
    if not counts:
        return "none"
    return ", ".join(f"{path}:{counts[path]}" for path in sorted(counts))


def print_summary(result: CensusResult) -> None:
    print("Mission authority census")
    print("Compatibility callers:")
    for token in sorted(EXPECTED_COMPATIBILITY):
        print(f"  {token}: {_format_locations(result.compatibility_calls[token])}")

    print("Production writer surfaces:")
    for token in sorted(WRITER_ALLOWLIST):
        locations = result.writer_occurrences[token]
        if locations:
            print(f"  {token}: {_format_locations(locations)}")

    exact_owner = sum(
        1
        for locations in result.exact_occurrences.values()
        for location in locations
        if location.path == AUTHORITY_PATH
    )
    exact_external = sum(
        1
        for locations in result.exact_occurrences.values()
        for location in locations
        if location.path != AUTHORITY_PATH
    )
    print(
        "Exact authority: "
        f"owner occurrences={exact_owner}, external production occurrences={exact_external}"
    )
    print(
        "Private MissionCom field assignments in state.rs: "
        f"{len(result.state_field_assignments)}"
    )
    print(
        "Private Mission leaf field assignments in leaf.rs: "
        f"{len(result.leaf_field_assignments)}"
    )


def print_matched_locations(result: CensusResult) -> None:
    print("Matched production census (path:line):", file=sys.stderr)
    groups: tuple[tuple[str, Mapping[str, list[Location]]], ...] = (
        ("compatibility", result.compatibility_calls),
        ("writer", result.writer_occurrences),
        ("exact", result.exact_occurrences),
    )
    for kind, locations_by_token in groups:
        for token in sorted(locations_by_token):
            for location in sorted(locations_by_token[token]):
                print(
                    f"  {kind} {token}: {location.path}:{location.line}",
                    file=sys.stderr,
                )
    for location in result.state_field_assignments:
        print(
            f"  raw MissionCom assignment: {location.path}:{location.line}",
            file=sys.stderr,
        )
    for location in result.leaf_field_assignments:
        print(
            f"  raw Mission leaf assignment: {location.path}:{location.line}",
            file=sys.stderr,
        )


def main() -> int:
    try:
        run_self_tests()
    except AssertionError as error:
        print(f"mission authority census self-test failed: {error}", file=sys.stderr)
        return 2

    if not SRC_ROOT.is_dir():
        print(f"source root not found: {SRC_ROOT}", file=sys.stderr)
        return 2

    result = scan_sources(_load_repo_sources())
    print_summary(result)
    if result.findings:
        print_matched_locations(result)
        print("Mission authority census FAILED:", file=sys.stderr)
        for finding in result.findings:
            position = (
                f"{finding.path}:{finding.line}"
                if finding.line
                else finding.path
            )
            print(
                f"  [{finding.code}] {position}: {finding.message}",
                file=sys.stderr,
            )
        return 1

    print("Mission authority census PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
