from __future__ import annotations

import json
import logging
from pathlib import Path
from typing import Any, Dict, List, Optional

from plugin_base import BasePlugin

PLUGIN_NAME = "delete_parts_of_data_set_only"
PLUGIN_DESCRIPTION = "Deletes files next to the entry based on mode token (export/compress/all), while keeping originals."
PLUGIN_TRIGGER = "manual"

log = logging.getLogger(__name__)


def _extract_path(payload: Dict[str, Any]) -> Optional[Path]:
    # Support both styles: entry events may provide mcap_path; schedule provides path/watch_dir
    for key in ("mcap_path", "path", "watch_dir"):
        v = payload.get(key)
        if isinstance(v, str) and v.strip():
            return Path(v.strip())
    return None


def _normalize_mode(raw: str) -> str:
    return (raw or "").strip().lower()


def _extract_mode_from_self_data(self_data: str) -> Optional[str]:
    """
    Mode comes from constructor-injected self.data.
    Accepts either plain string ("export") or JSON like {"mode":"export"}.
    """
    s = (self_data or "").strip()
    if not s:
        return None

    if s.startswith("{") and s.endswith("}"):
        try:
            obj = json.loads(s)
            if isinstance(obj, dict):
                for k in ("mode", "action", "kind", "type"):
                    v = obj.get(k)
                    if isinstance(v, str) and v.strip():
                        return _normalize_mode(v)
        except Exception:
            pass

    return _normalize_mode(s)


def _should_keep_in_all(file_path: Path, mcap_file_to_keep: Optional[Path]) -> bool:
    """
    Keep rules for mode=all:
    - keep the selected MCAP file (mcap_path) if provided
    - keep any *.mcap files
    - keep original YAML files:
        - keep metadata.yaml
        - keep any *.yaml/*.yml that does NOT have 'export' in filename
          (so *_export.yaml can be deleted, but metadata.yaml stays)
    """
    try:
        if mcap_file_to_keep is not None and file_path.resolve() == mcap_file_to_keep.resolve():
            return True
    except Exception:
        # if resolve fails, fall back to string compare
        if mcap_file_to_keep is not None and str(file_path) == str(mcap_file_to_keep):
            return True

    name_lc = file_path.name.lower()

    if file_path.suffix.lower() == ".mcap":
        return True

    if name_lc == "metadata.yaml":
        return True

    if file_path.suffix.lower() in (".yaml", ".yml") and "export" not in name_lc:
        return True

    return False


class PluginImpl(BasePlugin):
    def run(self, data: str) -> str:
        self.wait_while_paused()
        if self.should_stop():
            return "stopped"

        mode = _extract_mode_from_self_data(getattr(self, "data", "")) or ""
        if mode not in ("export", "compress", "all"):
            return json.dumps(
                {"ok": False, "reason": "invalid_mode", "mode": mode, "allowed": ["export", "compress", "all"]},
                ensure_ascii=False,
            )

        try:
            payload: Dict[str, Any] = json.loads(data) if data else {}
        except Exception as e:
            return json.dumps({"ok": False, "reason": "invalid_json_payload", "error": str(e)}, ensure_ascii=False)

        base_path = _extract_path(payload)
        if base_path is None:
            log.warning("missing 'mcap_path'/'path'/'watch_dir' in payload; nothing to delete")
            return json.dumps({"ok": True, "deleted": [], "skipped": [], "reason": "no_path"}, ensure_ascii=False)

        directory = base_path if base_path.is_dir() else base_path.parent
        if not directory.exists() or not directory.is_dir():
            return json.dumps(
                {"ok": True, "mode": mode, "dir": str(directory), "deleted": [], "skipped": [], "reason": "dir_missing"},
                ensure_ascii=False,
            )

        mcap_to_keep: Optional[Path] = None
        mcap_raw = payload.get("mcap_path")
        if isinstance(mcap_raw, str) and mcap_raw.strip():
            mcap_to_keep = Path(mcap_raw.strip())

        deleted: List[str] = []
        skipped: List[Dict[str, str]] = []

        # NEW: Fortschritt vorbereiten
        all_files = [p for p in directory.iterdir() if p.is_file()]
        total = max(1, len(all_files))
        self.report_progress(0.0, f"Scanning {total} files")

        for idx, p in enumerate(all_files, start=1):
            if self.should_stop():
                return "stopped"
            self.wait_while_paused()

            name_lc = p.name.lower()

            should_delete = False
            if mode == "all":
                if _should_keep_in_all(p, mcap_to_keep):
                    self.report_progress(idx / total, f"Keeping {p.name}")
                    continue
                should_delete = True
            elif mode == "export":
                should_delete = "export" in name_lc
            elif mode == "compress":
                should_delete = "compress" in name_lc

            if not should_delete:
                self.report_progress(idx / total, f"Skipping {p.name}")
                continue

            try:
                p.unlink()
                deleted.append(str(p))
                self.report_progress(idx / total, f"Deleted {p.name}")
            except Exception as e:
                skipped.append({"path": str(p), "reason": str(e)})
                self.report_progress(idx / total, f"Failed {p.name}")

        self.report_progress(1.0, "Done")

        return json.dumps(
            {"ok": True, "mode": mode, "dir": str(directory), "deleted": deleted, "skipped": skipped},
            ensure_ascii=False,
        )
