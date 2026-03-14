from __future__ import annotations

import json
import logging
import shutil
from pathlib import Path
from typing import Any, Dict, List

from plugin_base import BasePlugin

PLUGIN_NAME = "seed_test_data"
PLUGIN_DESCRIPTION = (
    "Seeds test data by copying MCAP/YAML recursively into /data "
    "(which is mounted to the project ./test_data)."
)
PLUGIN_TRIGGER = "manual"  # alternativ z. B. "on_schedule: */5 * * * *"

log = logging.getLogger(__name__)

# Voreinstellung für die Datenquelle:
# - "host"    -> /host_data
# - "project" -> /test_data_source
SOURCE_MODE = "host"
HOST_SOURCE_DIR = Path("/host_data")
PROJECT_SOURCE_DIR = Path("/test_data_source")
DEST_DIR = Path("/data")


def _is_interesting(p: Path) -> bool:
    """
    Prüft, ob eine Datei kopiert werden soll.

    Interessant sind nur echte Dateien mit Endung:
    - .mcap
    - .yaml
    - .yml
    """
    if not p.is_file():
        return False
    suf = p.suffix.lower()
    return suf in (".mcap", ".yaml", ".yml")


class PluginImpl(BasePlugin):
    def _copy_file_atomic_cooperative(
        self, src: Path, dst: Path, chunk_bytes: int
    ) -> None:
        """
        Kopiert eine Datei atomar und kooperativ.

        Atomar bedeutet hier:
        - zuerst in eine temporäre `*.partial`-Datei schreiben
        - am Ende per `replace()` an den finalen Ort verschieben

        Kooperativ bedeutet:
        - während des Kopierens auf Pause/Stop reagieren
        """
        dst.parent.mkdir(parents=True, exist_ok=True)
        tmp = dst.with_suffix(dst.suffix + ".partial")

        # Eventuelle alte temporäre Datei entfernen.
        try:
            tmp.unlink(missing_ok=True)
        except Exception:
            pass

        total = src.stat().st_size
        copied = 0

        log.info("copy start: %s -> %s (%d bytes)", src, dst, total)

        with open(src, "rb") as fsrc, open(tmp, "wb") as fdst:
            while True:
                # Bei Pause hier kooperativ blockieren.
                self.wait_while_paused()

                # Bei Stop die temporäre Datei aufräumen und sauber abbrechen.
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

                # Nächstes Chunk lesen.
                buf = fsrc.read(chunk_bytes)
                if not buf:
                    break

                fdst.write(buf)
                copied += len(buf)

                # Gelegentliche Fortschrittslogs bei großen Dateien.
                if copied % (64 * 1024 * 1024) < chunk_bytes:
                    log.info(
                        "copy progress: %s -> %.1f%%",
                        src.name,
                        (copied / max(total, 1)) * 100.0,
                    )

            fdst.flush()

        # Datei-Metadaten möglichst übernehmen.
        try:
            shutil.copystat(src, tmp, follow_symlinks=True)
        except Exception:
            pass

        # Atomarer Austausch:
        # Erst jetzt erscheint die Datei am Ziel vollständig.
        tmp.replace(dst)
        log.info("copy done: %s", dst)

    def run(self, data: str) -> str:
        """
        Kopiert Testdaten rekursiv in das Zielverzeichnis.

        Optionales JSON-Payload:
        {
          "dry_run": false,
          "limit": 0,
          "overwrite": false,
          "source_mode": "host",
          "chunk_bytes": 1048576
        }

        Bedeutung:
        - dry_run: nur simulieren, nicht wirklich kopieren
        - limit: maximal so viele Dateien kopieren
        - overwrite: vorhandene Dateien überschreiben
        - source_mode: Quelle "host" oder "project"
        - chunk_bytes: Blockgröße fürs Kopieren
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

        # Chunkgröße lesen und sinnvoll begrenzen.
        chunk_bytes = int(
            payload.get("chunk_bytes", 1024 * 1024) or (1024 * 1024)
        )
        chunk_bytes = max(
            64 * 1024, min(chunk_bytes, 16 * 1024 * 1024)
        )

        mode = str(payload.get("source_mode", SOURCE_MODE)).strip().lower()
        if mode not in ("host", "project"):
            return "error: source_mode must be 'host' or 'project'"

        source_dir = (
            HOST_SOURCE_DIR if mode == "host" else PROJECT_SOURCE_DIR
        ).resolve()
        dest_dir = DEST_DIR.resolve()

        # Quellen/Ziele validieren.
        if not source_dir.exists() or not source_dir.is_dir():
            return f"error: source_dir not found or not a directory: {source_dir}"
        if not dest_dir.exists() or not dest_dir.is_dir():
            return f"error: dest_dir not found or not a directory: {dest_dir}"

        # Rekursiv alle interessanten Dateien sammeln.
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

            # Bereits vorhandene Datei überspringen, wenn overwrite=False.
            if dst.exists() and not overwrite:
                skipped.append(
                    {"src": str(src), "dst": str(dst), "reason": "already_exists"}
                )
                continue

            # Optionales Limit beachten.
            if limit > 0 and count_copied >= limit:
                skipped.append(
                    {"src": str(src), "dst": str(dst), "reason": "limit_reached"}
                )
                continue

            try:
                if not dry_run:
                    self._copy_file_atomic_cooperative(src, dst, chunk_bytes)

                copied.append(str(dst))
                count_copied += 1
            except Exception as e:
                # Stop wird bewusst als sauberer Status behandelt.
                if str(e) == "stopped":
                    return "stopped"

                errors.append({"src": str(src), "dst": str(dst), "error": str(e)})

        # Ergebnis als JSON-String zurückgeben.
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
