"""Ranking helpers for evidence search."""

from __future__ import annotations


SOURCE_WEIGHTS = {
    "ghidra": 1.00,
    "trace": 0.92,
    "contract": 0.88,
    "synthesis": 0.76,
    "audit": 0.68,
    "ini": 0.66,
    "plan": 0.45,
    "unknown": 0.30,
}

STATUS_WEIGHTS = {
    "verified": 1.00,
    "synthesis": 0.78,
    "plan": 0.52,
    "unknown": 0.40,
    "stale": 0.10,
}

AUTHORITY_SOURCE_WEIGHTS = {
    "ghidra": 3.00,
    "trace": 2.70,
    "contract": 2.55,
    "synthesis": 1.95,
    "audit": 1.60,
    "ini": 1.45,
    "plan": 0.75,
    "unknown": 0.55,
}

AUTHORITY_STATUS_WEIGHTS = {
    "verified": 2.50,
    "synthesis": 1.65,
    "plan": 0.85,
    "unknown": 0.55,
    "stale": -1.50,
}


def evidence_weight(source_kind: str, status: str) -> float:
    return SOURCE_WEIGHTS.get(source_kind, SOURCE_WEIGHTS["unknown"]) + STATUS_WEIGHTS.get(status, STATUS_WEIGHTS["unknown"])


def related_evidence_weight(source_kind: str, status: str) -> float:
    return 0.35 * SOURCE_WEIGHTS.get(source_kind, SOURCE_WEIGHTS["unknown"]) + 0.35 * STATUS_WEIGHTS.get(status, STATUS_WEIGHTS["unknown"])


def evidence_order_sql(alias: str = "d") -> str:
    return f"""
    CASE {alias}.source_kind
      WHEN 'ghidra' THEN 7
      WHEN 'trace' THEN 6
      WHEN 'contract' THEN 5
      WHEN 'synthesis' THEN 4
      WHEN 'audit' THEN 3
      WHEN 'ini' THEN 2
      WHEN 'plan' THEN 1
      ELSE 0
    END DESC,
    CASE {alias}.status
      WHEN 'verified' THEN 4
      WHEN 'synthesis' THEN 3
      WHEN 'plan' THEN 2
      WHEN 'unknown' THEN 1
      ELSE 0
    END DESC
    """


def final_score(bm25: float, source_kind: str, status: str, exact_boost: float = 0.0) -> float:
    # SQLite bm25 is lower for better matches and is often negative. Convert the
    # magnitude into a bounded positive score, then add evidence weighting so
    # verified reports win close calls.
    lexical = min(1.0, max(-bm25, 0.0) / 8.0)
    return lexical + evidence_weight(source_kind, status) + exact_boost


def authority_base_score(source_kind: str, status: str) -> float:
    return AUTHORITY_SOURCE_WEIGHTS.get(source_kind, AUTHORITY_SOURCE_WEIGHTS["unknown"]) + AUTHORITY_STATUS_WEIGHTS.get(
        status, AUTHORITY_STATUS_WEIGHTS["unknown"]
    )


def authority_score(
    source_kind: str,
    status: str,
    exact_anchor_hits: int,
    partial_anchor_hits: int,
    lexical_hits: int,
    has_handoff: bool,
    has_correction_signal: bool,
    risk_flags: list[str],
    recency_bonus: float = 0.0,
) -> float:
    score = authority_base_score(source_kind, status)
    score += min(4.5, exact_anchor_hits * 1.15)
    score += min(2.0, partial_anchor_hits * 0.45)
    score += min(3.0, lexical_hits * 0.25)
    if has_handoff:
        score += 1.25
    if has_correction_signal:
        score += 0.40
    score += recency_bonus

    if any(flag in risk_flags for flag in ("stale/superseded", "legacy gate wording")):
        score -= 2.75
    if "wrong/misleading wording" in risk_flags:
        score -= 1.25
    if "unchecked/deferred uncertainty" in risk_flags:
        score -= 0.45
    return score
