from __future__ import annotations

import json
import logging
import shutil
from pathlib import Path
from typing import Any, Dict, List

from plugin_base import BasePlugin

PLUGIN_NAME = "seed_test_data_plugin"
PLUGIN_DESCRIPTION = (
    "Seeds test data by copying MCAP/YAML recursively into /data "
    "(which is mounted to the project ./test_data)."
)
PLUGIN_TRIGGER = "manual"  # or: "on_schedule: */5 * * * *"

log = logging.getLogger(__name__)

SOURCE_MODE = "host"  # "host" | "project"
HOST_SOURCE_DIR = Path("/host_data")
PROJECT_SOURCE_DIR = Path("/test_data_source")
DEST_DIR = Path("/data")


def _is_interesting(p: Path) -> bool:
    if not p.is_file():
        return False
    suf = p.suffix.lower()
    return suf in (".mcap", ".yaml", ".yml")


class PluginImpl(BasePlugin):
    def _copy_file_atomic_cooperative(self, src: Path, dst: Path, chunk_bytes: int) -> None:
        """
        Atomic copy using *.partial + replace(), but cooperatively checks pause/stop during the copy.
        """
        dst.parent.mkdir(parents=True, exist_ok=True)
        tmp = dst.with_suffix(dst.suffix + ".partial")

        try:
            tmp.unlink(missing_ok=True)
        except Exception:
            pass

        total = src.stat().st_size
        copied = 0

        log.info("copy start: %s -> %s (%d bytes)", src, dst, total)

        with open(src, "rb") as fsrc, open(tmp, "wb") as fdst:
            while True:
                self.wait_while_paused()
                if self.should_stop():
                    try:
                        fdst.flush()
                    finally:
                        fdst.close()
                    try:
                        tmp.unlink(missing_ok=True)
                    except Exception:
                        pass
                    raise RuntimeError("stopped")

                buf = fsrc.read(chunk_bytes)
                if not buf:
                    break
                fdst.write(buf)
                copied += len(buf)

                # light progress log every ~64 MiB
                if copied % (64 * 1024 * 1024) < chunk_bytes:
                    log.info("copy progress: %s -> %.1f%%", src.name, (copied / max(total, 1)) * 100.0)

            fdst.flush()

        try:
            shutil.copystat(src, tmp, follow_symlinks=True)
        except Exception:
            pass

        tmp.replace(dst)
        log.info("copy done: %s", dst)

    def run(self, data: str) -> str:
        """
        Optional JSON payload:
        {
          "dry_run": false,
          "limit": 0,
          "overwrite": false,
          "source_mode": "host",
          "chunk_bytes": 1048576
        }
        """
        self.wait_while_paused()
        if self.should_stop():
            return "stopped"

        payload: Dict[str, Any] = {}
        if data:
            try:
                payload = json.loads(data)
            except Exception as e:
                return f"error: invalid json payload: {e}"

        dry_run = bool(payload.get("dry_run", False))
        limit = int(payload.get("limit", 0) or 0)
        overwrite = bool(payload.get("overwrite", False))
        chunk_bytes = int(payload.get("chunk_bytes", 1024 * 1024) or (1024 * 1024))  # default 1 MiB
        chunk_bytes = max(64 * 1024, min(chunk_bytes, 16 * 1024 * 1024))  # clamp 64 KiB..16 MiB

        mode = str(payload.get("source_mode", SOURCE_MODE)).strip().lower()
        if mode not in ("host", "project"):
            return "error: source_mode must be 'host' or 'project'"

        source_dir = (HOST_SOURCE_DIR if mode == "host" else PROJECT_SOURCE_DIR).resolve()
        dest_dir = DEST_DIR.resolve()

        if not source_dir.exists() or not source_dir.is_dir():
            return f"error: source_dir not found or not a directory: {source_dir}"
        if not dest_dir.exists() or not dest_dir.is_dir():
            return f"error: dest_dir not found or not a directory: {dest_dir}"

        files = [p for p in source_dir.rglob("*") if _is_interesting(p)]
        if not files:
            return f"error: no .mcap/.yaml files found in source_dir (recursive): {source_dir}"

        copied: List[str] = []
        skipped: List[Dict[str, str]] = []
        errors: List[Dict[str, str]] = []

        count_copied = 0
        for i, src in enumerate(files, start=1):
            self.wait_while_paused()
            if self.should_stop():
                return "stopped"

            rel = src.relative_to(source_dir)
            dst = dest_dir / rel

            log.info("file %d/%d: %s", i, len(files), rel)

            if dst.exists() and not overwrite:
                skipped.append({"src": str(src), "dst": str(dst), "reason": "already_exists"})
                continue

            if limit > 0 and count_copied >= limit:
                skipped.append({"src": str(src), "dst": str(dst), "reason": "limit_reached"})
                continue

            try:
                if not dry_run:
                    self._copy_file_atomic_cooperative(src, dst, chunk_bytes)
                copied.append(str(dst))
                count_copied += 1
            except Exception as e:
                # if we raised "stopped" above, propagate a clean status
                if str(e) == "stopped":
                    return "stopped"
                errors.append({"src": str(src), "dst": str(dst), "error": str(e)})

        return json.dumps(
            {
                "ok": True,
                "mode": mode,
                "source_dir": str(source_dir),
                "dest_dir": str(dest_dir),
                "dry_run": dry_run,
                "overwrite": overwrite,
                "chunk_bytes": chunk_bytes,
                "copied_count": len(copied),
                "skipped_count": len(skipped),
                "errors_count": len(errors),
            },
            ensure_ascii=False,
        )