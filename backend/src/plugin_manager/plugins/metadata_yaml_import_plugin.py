# metadata_yaml_import_plugin.py
from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, Optional

from plugin_base import BasePlugin

PLUGIN_NAME = "metadata_yaml_import_plugin"
PLUGIN_DESCRIPTION = ("Imports YAML metadata when only the metadata file is "
                      "added (tries to auto-match an MCAP in the same folder).")
PLUGIN_TRIGGER = "manual"


def _resolve_mcap_path(
        metadata_path: Path,
    mcap_path_raw: str,
    mcap_filename: str,
) -> Optional[Path]:
    # If explicit mcap_path provided, use it
    if mcap_path_raw:
        p = Path(mcap_path_raw)
        if not p.exists():
            raise RuntimeError(f"mcap_path does not exist: {p}")
        if p.suffix.lower() != ".mcap":
            raise RuntimeError(f"mcap_path must point to a .mcap file: {p}")
        return p

    # Otherwise: search same folder
    folder = metadata_path.parent
    mcaps = sorted(folder.glob("*.mcap"))

    if not mcaps:
        return None  # "only metadata file added" and no mcap yet

    if mcap_filename:
        chosen = folder / mcap_filename
        if not chosen.exists():
            raise RuntimeError(
                f"mcap_filename was provided but not found in folder: {chosen}"
            )
        if chosen.suffix.lower() != ".mcap":
            raise RuntimeError(f"mcap_filename must end with .mcap: {chosen}")
        return chosen

    if len(mcaps) == 1:
        return mcaps[0]

    raise RuntimeError(
        "ambiguous: multiple .mcap files found next to metadata. "
        "Provide 'mcap_filename' or 'mcap_path'. "
        f"Found: {[str(p.name) for p in mcaps]}"
    )


class PluginImpl(BasePlugin):
    """
    Expected run(data) JSON payload (string):

    {
      "metadata_path": "C:/path/to/file.yaml",

      // optional: if you already know the mcap
      "mcap_path": "C:/path/to/file.mcap",

      // optional: if multiple mcaps exist in folder, pick one by filename
      "mcap_filename": "recording.mcap",

      // optional: if true, return the parsed YAML (as JSON) in the result string (can be big)
      "echo_parsed": false
    }

    Behavior:
    - If only metadata_path is provided, the plugin searches the metadata file's folder for *.mcap.
      - 0 found  -> returns "pending: no mcap found yet"
      - 1 found  -> uses it
      - >1 found -> returns "error: ambiguous mcaps" (unless mcap_filename provided)
    - Parses YAML and returns a compact summary.
    """

    def run(self, data: str) -> str:
        # Cooperative pause/stop at entry
        self.wait_while_paused()
        if self.should_stop():
            return "stopped"

        try:
            payload = json.loads(data) if data else {}
        except Exception as e:
            return f"error: invalid json payload: {e}"

        metadata_path_raw = (payload.get("metadata_path") or "").strip()
        if not metadata_path_raw:
            return "error: missing 'metadata_path'"

        metadata_path = Path(metadata_path_raw)
        if not metadata_path.exists():
            return f"error: metadata file does not exist: {metadata_path}"
        if metadata_path.suffix.lower() not in (".yaml", ".yml"):
            return f"error: metadata file must be .yaml/.yml: {metadata_path}"

        mcap_path = _resolve_mcap_path(
            metadata_path=metadata_path,
            mcap_path_raw=(payload.get("mcap_path") or "").strip(),
            mcap_filename=(payload.get("mcap_filename") or "").strip(),
        )
        if mcap_path is None:
            # This is the "only metadata file added" case where no mcap exists yet
            return "pending: no mcap found yet (only metadata present)"
            # TODO Verhalten richtig?

        # Parse YAML
        parsed = self._parse_yaml(metadata_path)
        if parsed is None:
            # _parse_yaml returns None only if stop requested mid-way (cooperative)
            return "stopped"

        # Optional: echo parsed YAML (can be large)
        echo_parsed = bool(payload.get("echo_parsed", False))

        # Build a small summary that is stable and useful
        summary = {
            "metadata_path": str(metadata_path),
            "mcap_path": str(mcap_path),
            "top_level_keys": list(parsed.keys()) if isinstance(parsed, dict) else None,
            "has_definitions": isinstance(parsed, dict) and "definitions" in parsed,
        }

        if echo_parsed:
            return json.dumps({"summary": summary, "metadata": parsed}, ensure_ascii=False)
        return json.dumps({"summary": summary}, ensure_ascii=False)

    def _parse_yaml(self, metadata_path: Path) -> Optional[Dict[str, Any] | Any]:
        # stop/pause checkpoint
        self.wait_while_paused()
        if self.should_stop():
            return None

        try:
            import yaml
        except Exception as e:
            raise RuntimeError(
                "Python dependency missing: cannot import 'yaml' (PyYAML). "
                f"Details: {e}"
            )

        # Read + parse
        text = metadata_path.read_text(encoding="utf-8", errors="replace")

        # stop/pause checkpoint
        self.wait_while_paused()
        if self.should_stop():
            return None

        try:
            return yaml.safe_load(text)
        except Exception as e:
            raise RuntimeError(f"failed to parse yaml: {e}")
