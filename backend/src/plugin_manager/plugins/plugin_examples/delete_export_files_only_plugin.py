from __future__ import annotations

import json
import logging
from pathlib import Path
from typing import Any, Dict, List, Optional

from plugin_base import BasePlugin

PLUGIN_NAME = "delete_export_files_only"
PLUGIN_DESCRIPTION = "Deletes only *_export.* files (entry-local on events, recursive on schedule)."
PLUGIN_TRIGGER = "manual"  # example: every 10 seconds

log = logging.getLogger(__name__)


def _extract_path(payload: Dict[str, Any]) -> Optional[Path]:
    # Support both styles: entry events may provide mcap_path; schedule provides path/watch_dir
    for key in ("mcap_path", "path", "watch_dir"):
        v = payload.get(key)
        if isinstance(v, str) and v.strip():
            return Path(v.strip())
    return None


def _is_export_file(p: Path) -> bool:
    return p.is_file() and p.stem.endswith("_export")


class PluginImpl(BasePlugin):
    def run(self, data: str) -> str:
        self.wait_while_paused()
        if self.should_stop():
            return "stopped"

        try:
            payload: Dict[str, Any] = json.loads(data) if data else {}
        except Exception as e:
            return f"error: invalid json payload: {e}"

        event = str(payload.get("event", "") or "")
        base_path = _extract_path(payload)
        if base_path is None:
            log.warning("missing 'mcap_path'/'path'/'watch_dir' in payload; nothing to delete")
            return json.dumps({"ok": True, "deleted": [], "skipped": [], "reason": "no_path"}, ensure_ascii=False)

        deleted: List[str] = []
        skipped: List[Dict[str, str]] = []

        # Schedule: treat base_path as directory root and delete recursively
        if event == "schedule":
            root = base_path
            if root.is_file():
                root = root.parent

            if not root.exists() or not root.is_dir():
                return json.dumps(
                    {"ok": True, "deleted": [], "skipped": [], "reason": "root_not_dir", "root": str(root)},
                    ensure_ascii=False,
                )

            for p in root.rglob("*"):
                if self.should_stop():
                    return "stopped"
                self.wait_while_paused()

                if not _is_export_file(p):
                    continue

                try:
                    p.unlink()
                    deleted.append(str(p))
                except Exception as e:
                    skipped.append({"path": str(p), "reason": str(e)})

            return json.dumps(
                {"ok": True, "mode": "schedule_recursive", "root": str(root), "deleted": deleted, "skipped": skipped},
                ensure_ascii=False,
            )

        # Entry events: treat base_path as the entry file path (or directory) and delete only next to it
        entry_path = base_path
        directory = entry_path if entry_path.is_dir() else entry_path.parent

        if not directory.exists() or not directory.is_dir():
            return json.dumps(
                {"ok": True, "mode": "entry_local", "dir": str(directory), "deleted": [], "skipped": [], "reason": "dir_missing"},
                ensure_ascii=False,
            )

        for p in directory.iterdir():
            if self.should_stop():
                return "stopped"
            self.wait_while_paused()

            if not _is_export_file(p):
                continue

            try:
                p.unlink()
                deleted.append(str(p))
            except Exception as e:
                skipped.append({"path": str(p), "reason": str(e)})

        return json.dumps(
            {"ok": True, "mode": "entry_local", "dir": str(directory), "deleted": deleted, "skipped": skipped},
            ensure_ascii=False,
        )
