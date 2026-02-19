# metadata_yaml_export_plugin.py
from __future__ import annotations

import json
import time
from pathlib import Path
from typing import Any, Dict, Optional

from plugin_base import BasePlugin

PLUGIN_NAME = "metadata_yaml_export_plugin"
PLUGIN_DESCRIPTION = "Exports metadata (given as JSON) into a YAML file."
PLUGIN_TRIGGER = "manual"


def _resolve_output_path(output_path_raw: str, mcap_path_raw: str) -> Path:
    if output_path_raw:
        p = Path(output_path_raw)
        if p.suffix.lower() not in (".yaml", ".yml"):
            raise RuntimeError("output_path must end with .yaml or .yml")
        return p

    if mcap_path_raw:
        mcap = Path(mcap_path_raw)
        if not mcap.exists():
            raise RuntimeError(f"mcap_path does not exist: {mcap}")
        if mcap.suffix.lower() != ".mcap":
            raise RuntimeError(f"mcap_path must point to a .mcap file: {mcap}")
        return mcap.with_suffix(".yaml")

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
      "wrap_definitions": true,            // optional: wrap into {"definitions": metadata}
      "overwrite": false                   // optional: refuse if file exists
    }

    Rules:
    - If output_path is given -> write exactly there.
    - Else if mcap_path is given -> write next to it as "<mcap_stem>.yaml".
    - Else -> error.
    """

    def run(self, data: str) -> str:
        self.wait_while_paused()
        if self.should_stop():
            return "stopped"

        try:
            payload = json.loads(data) if data else {}
        except Exception as e:
            return f"error: invalid json payload: {e}"

        metadata = payload.get("metadata")
        if metadata is None:
            return "error: missing 'metadata'"

        if not isinstance(metadata, dict):
            return "error: 'metadata' must be a JSON object"

        output_path = _resolve_output_path(
            output_path_raw=(payload.get("output_path") or "").strip(),
            mcap_path_raw=(payload.get("mcap_path") or "").strip(),
        )

        overwrite = bool(payload.get("overwrite", False))
        wrap_definitions = bool(payload.get("wrap_definitions", False))

        to_dump: Dict[str, Any]
        if wrap_definitions:
            to_dump = {"definitions": metadata}
        else:
            to_dump = metadata

        try:
            yaml_text = _dump_yaml(to_dump)
        except Exception as e:
            return f"error: yaml export failed: {e}"

        try:
            self._write_atomic(output_path, yaml_text, overwrite=overwrite)
        except Exception as e:
            return f"error: failed to write yaml: {e}"

        return json.dumps(
            {
                "ok": True,
                "output_path": str(output_path),
                "bytes": len(yaml_text.encode("utf-8", errors="replace")),
            },
            ensure_ascii=False,
        )

    def _write_atomic(self, path: Path, text: str, overwrite: bool) -> None:
        self.wait_while_paused()
        if self.should_stop():
            raise RuntimeError("stopped")

        path.parent.mkdir(parents=True, exist_ok=True)

        if path.exists() and not overwrite:
            raise RuntimeError(f"output file already exists (set overwrite=true): {path}")

        tmp = path.with_suffix(path.suffix + f".partial.{int(time.time() * 1000)}")
        tmp.write_text(text, encoding="utf-8", errors="replace")

        self.wait_while_paused()
        if self.should_stop():
            try:
                tmp.unlink(missing_ok=True)
            except Exception:
                pass
            raise RuntimeError("stopped")

        # Atomic replace on Windows works with Path.replace (overwrites if exists)
        tmp.replace(path)
