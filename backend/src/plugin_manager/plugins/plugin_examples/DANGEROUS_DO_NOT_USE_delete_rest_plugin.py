from __future__ import annotations

import json
import logging
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

from plugin_base import BasePlugin

PLUGIN_NAME = "DANGEROUS_DO_NOT_USE_delete_rest_plugin"
PLUGIN_DESCRIPTION = (
    "Deletes associated YAML and *_export.yaml files when an entry (MCAP) is deleted."
)
PLUGIN_TRIGGER = "on_entry_delete"

log = logging.getLogger(__name__)


def _append_export_suffix(p: Path) -> Path:
    if p.stem.endswith("_export"):
        return p
    return p.with_name(f"{p.stem}_export{p.suffix}")


def _safe_unlink(p: Path) -> Tuple[bool, str]:
    """
    Returns (deleted, status_string).
    status_string is one of: "deleted", "missing", "is_dir", "error: ..."
    """
    try:
        if not p.exists():
            return False, "missing"
        if p.is_dir():
            return False, "is_dir"
        p.unlink()
        return True, "deleted"
    except Exception as e:
        return False, f"error: {e}"


def _extract_mcap_path(payload: Dict[str, Any]) -> Optional[Path]:
    # Prefer explicit fields if present
    for key in ("mcap_path", "path"):
        v = payload.get(key)
        if isinstance(v, str) and v.strip():
            return Path(v.strip())
    return None


def _candidate_yaml_paths(mcap_path: Path) -> List[Path]:
    """
    Given /x/foo.mcap, return likely yaml files to delete:
    - /x/foo.yaml
    - /x/foo_export.yaml
    - /x/metadata.yaml  (common convention in your test data)
    - /x/metadata_export.yaml (just in case)
    """
    base_yaml = mcap_path.with_suffix(".yaml")
    return [
        base_yaml,
        _append_export_suffix(base_yaml),
        mcap_path.with_name("metadata.yaml"),
        _append_export_suffix(mcap_path.with_name("metadata.yaml")),
    ]


class PluginImpl(BasePlugin):
    def run(self, data: str) -> str:
        log.debug("run() raw data (first 500 chars): %r", (data[:500] if data else ""))

        self.wait_while_paused()
        if self.should_stop():
            return "stopped"

        try:
            payload: Dict[str, Any] = json.loads(data) if data else {}
        except Exception as e:
            log.error("invalid json payload: %s", e)
            return f"error: invalid json payload: {e}"

        mcap_path = _extract_mcap_path(payload)
        if mcap_path is None:
            log.warning("missing 'path'/'mcap_path' in payload; nothing to delete")
            return json.dumps(
                {"ok": True, "deleted": [], "skipped": [], "reason": "no_path"},
                ensure_ascii=False,
            )

        # If a directory path is sent (shouldn't happen), bail out safely
        if mcap_path.exists() and mcap_path.is_dir():
            msg = f"provided path is a directory, refusing: {mcap_path}"
            log.warning(msg)
            return f"error: {msg}"

        deleted: List[str] = []
        skipped: List[Dict[str, str]] = []

        # Delete YAMLs that belong to this MCAP
        for p in _candidate_yaml_paths(mcap_path):
            if self.should_stop():
                return "stopped"
            self.wait_while_paused()

            ok, status = _safe_unlink(p)
            if ok:
                log.info("deleted %s", p)
                deleted.append(str(p))
            else:
                # "missing" is normal, so log it only on debug
                if status == "missing":
                    log.debug("not found (skip): %s", p)
                else:
                    log.warning("skip %s (%s)", p, status)
                skipped.append({"path": str(p), "status": status})

        return json.dumps(
            {
                "ok": True,
                "mcap_path": str(mcap_path),
                "deleted": deleted,
                "skipped": skipped,
            },
            ensure_ascii=False,
        )
