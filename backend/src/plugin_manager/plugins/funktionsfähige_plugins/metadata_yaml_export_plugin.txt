# metadata_yaml_export_plugin.txt
from __future__ import annotations

import json
import time
import logging
from pathlib import Path
from typing import Any, Dict, Optional

from plugin_base import BasePlugin

PLUGIN_NAME = "metadata_yaml_export_plugin"
PLUGIN_DESCRIPTION = "Exports metadata (given as JSON) into a YAML file."
PLUGIN_TRIGGER = "manual"

log = logging.getLogger(__name__)


def _append_export_suffix(p: Path) -> Path:
    """
    Ensure the filename ends with '_export' (before the extension).
    - file.yaml         -> file_export.yaml
    - file_export.yaml  -> file_export.yaml
    """
    if p.stem.endswith("_export"):
        return p
    return p.with_name(f"{p.stem}_export{p.suffix}")


def _resolve_output_path(output_path_raw: str, mcap_path_raw: str) -> Path:
    if output_path_raw:
        p = Path(output_path_raw)
        if p.suffix.lower() not in (".yaml", ".yml"):
            raise RuntimeError("output_path must end with .yaml or .yml")
        return _append_export_suffix(p)

    if mcap_path_raw:
        mcap = Path(mcap_path_raw)
        if not mcap.exists():
            raise RuntimeError(f"mcap_path does not exist: {mcap}")
        if mcap.suffix.lower() != ".mcap":
            raise RuntimeError(f"mcap_path must point to a .mcap file: {mcap}")

        yaml_out = mcap.with_suffix(".yaml")
        return _append_export_suffix(yaml_out)

    raise RuntimeError("either 'output_path' or 'mcap_path' must be provided")


def _dump_yaml(obj: Dict[str, Any]) -> str:
    try:
        import yaml  # type: ignore
    except Exception as e:
        raise RuntimeError(
            "Python dependency missing: cannot import 'yaml' (PyYAML). "
            f"Details: {e}"
        )

    # Keep YAML readable & stable
    return yaml.safe_dump(
        obj,
        sort_keys=False,
        allow_unicode=True,
        default_flow_style=False,
        width=120,
    )


class PluginImpl(BasePlugin):
    """
    Expected run(data) JSON payload (string), e.g.:

    {
      "metadata": { ... },                 // required (any JSON object)
      "output_path": "C:/.../meta.yaml",   // optional
      "mcap_path": "C:/.../file.mcap",     // optional alternative to output_path
      "wrap_definitions": true             // optional: wrap into {"definitions": metadata}
    }

    Rules:
    - Output file is always written next to the given path, but with filename suffix "_export".
    - If the _export file already exists, it is deleted and replaced (always overwrite behavior).
    """

    def run(self, data: str) -> str:
        log.debug(
            "run() called: data_bytes=%d",
            len(data.encode("utf-8", errors="replace")) if data else 0,
        )
        log.debug("run() raw data (first 500 chars): %r", (data[:500] if data else ""))

        self.wait_while_paused()
        if self.should_stop():
            log.info("stop requested before processing")
            return "stopped"

        try:
            payload = json.loads(data) if data else {}
        except Exception as e:
            log.error("invalid json payload: %s", e)
            return f"error: invalid json payload: {e}"

        log.debug("payload keys: %s", sorted(list(payload.keys())))

        metadata = payload.get("metadata")
        if metadata is None:
            log.warning("missing 'metadata' in payload")
            return "error: missing 'metadata'"

        if not isinstance(metadata, dict):
            log.warning("'metadata' is not an object: type=%s", type(metadata).__name__)
            return "error: 'metadata' must be a JSON object"

        output_path = _resolve_output_path(
            output_path_raw=(payload.get("output_path") or "").strip(),
            mcap_path_raw=(payload.get("mcap_path") or "").strip(),
        )
        log.info("resolved output_path=%s", output_path)

        wrap_definitions = bool(payload.get("wrap_definitions", False))
        log.debug("options: wrap_definitions=%s", wrap_definitions)

        to_dump: Dict[str, Any]
        if wrap_definitions:
            to_dump = {"definitions": metadata}
        else:
            to_dump = metadata

        try:
            yaml_text = _dump_yaml(to_dump)
        except Exception as e:
            log.error("yaml export failed: %s", e)
            return f"error: yaml export failed: {e}"

        try:
            # Always overwrite: if output exists, delete it first
            self._write_atomic_overwrite(output_path, yaml_text)
        except Exception as e:
            log.error("failed to write yaml: %s", e)
            return f"error: failed to write yaml: {e}"

        log.info(
            "export OK: path=%s bytes=%d",
            output_path,
            len(yaml_text.encode("utf-8", errors="replace")),
        )

        return json.dumps(
            {
                "ok": True,
                "output_path": str(output_path),
                "bytes": len(yaml_text.encode("utf-8", errors="replace")),
            },
            ensure_ascii=False,
        )

    def _write_atomic_overwrite(self, path: Path, text: str) -> None:
        """
        Always overwrite behavior:
        - If the target exists, remove it first.
        - Write to a unique .partial file, then replace into place atomically.
        """
        self.wait_while_paused()
        if self.should_stop():
            raise RuntimeError("stopped")

        path.parent.mkdir(parents=True, exist_ok=True)

        if path.exists():
            try:
                path.unlink()
                log.debug("deleted existing export file before writing: %s", path)
            except Exception as e:
                raise RuntimeError(f"failed to delete existing output file: {path} ({e})")

        tmp = path.with_suffix(path.suffix + f".partial.{int(time.time() * 1000)}")
        tmp.write_text(text, encoding="utf-8", errors="replace")

        self.wait_while_paused()
        if self.should_stop():
            try:
                tmp.unlink(missing_ok=True)
            except Exception:
                pass
            raise RuntimeError("stopped")

        # Atomic replace (also works on Windows). If something recreated path in between, replace will overwrite.
        tmp.replace(path)