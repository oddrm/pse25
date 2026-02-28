# metadata_yaml_import_plugin.py
from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Optional
from urllib.parse import quote

from plugin_base import BasePlugin

PLUGIN_NAME = "metadata_yaml_import_plugin"
PLUGIN_DESCRIPTION = (
    "Imports YAML metadata when only the metadata file is added "
    "(tries to auto-match an MCAP in the same folder)."
)
PLUGIN_TRIGGER = "manual"


def _find_metadata_yaml_next_to_mcap(mcap_path: Path, metadata_filename: str) -> Optional[Path]:
    folder = mcap_path.parent
    yamls = sorted(list(folder.glob("*.yaml")) + list(folder.glob("*.yml")))
    if len(yamls) == 0:
        return None

    # 1) If caller specifies a filename, use it (strict)
    if metadata_filename:
        chosen = folder / metadata_filename
        if not chosen.exists():
            raise RuntimeError(f"metadata_filename was provided but not found in folder: {chosen}")
        if chosen.suffix.lower() not in (".yaml", ".yml"):
            raise RuntimeError(f"metadata_filename must end with .yaml/.yml: {chosen}")
        return chosen

    # 2) Prefer conventional names
    for preferred in ("metadata.yaml", "metadata.yml"):
        p = folder / preferred
        if p.exists():
            return p

    # 3) Prefer YAML matching MCAP stem
    stem = mcap_path.stem  # e.g. excavator_drive
    for ext in (".yaml", ".yml"):
        p = folder / f"{stem}{ext}"
        if p.exists():
            return p

    # 4) If only one exists, use it
    if len(yamls) == 1:
        return yamls[0]

    raise RuntimeError(
        "ambiguous: multiple YAML files found next to MCAP. "
        "Provide 'metadata_path' (or 'metadata_filename'). "
        f"Found: {[p.name for p in yamls]}"
    )


def _resolve_mcap_path(
    metadata_path: Path,
    mcap_path_raw: str,
    mcap_filename: str,
) -> Optional[Path]:
    if mcap_path_raw:
        p = Path(mcap_path_raw)
        if not p.exists():
            raise RuntimeError(f"mcap_path does not exist: {p}")
        if p.suffix.lower() != ".mcap":
            raise RuntimeError(f"mcap_path must point to a .mcap file: {p}")
        return p

    folder = metadata_path.parent
    mcaps = sorted(folder.glob("*.mcap"))
    if not mcaps:
        return None

    if mcap_filename:
        chosen = folder / mcap_filename
        if not chosen.exists():
            raise RuntimeError(f"mcap_filename was provided but not found in folder: {chosen}")
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


def _find_single_yaml_next_to_mcap(mcap_path: Path) -> Optional[Path]:
    folder = mcap_path.parent
    yamls = sorted(list(folder.glob("*.yaml")) + list(folder.glob("*.yml")))

    # NEW: ignore generated export YAMLs like "*_export.yaml"
    yamls = [p for p in yamls if not p.stem.lower().endswith("_export")]

    if len(yamls) == 0:
        return None
    if len(yamls) == 1:
        return yamls[0]
    raise RuntimeError(
        "ambiguous: multiple YAML files found next to MCAP. "
        "Provide 'metadata_path'. "
        f"Found: {[p.name for p in yamls]}"
    )


def _import_output_path(metadata_path: Path) -> Path:
    # metadata.yaml -> metadata_import.yaml
    # metadata.yml  -> metadata_import.yml
    return metadata_path.with_name(f"{metadata_path.stem}_import{metadata_path.suffix}")


def _find_export_yaml_next_to_mcap(mcap_path: Path) -> Optional[Path]:
    """
    Prefer a single '*_export.yaml|yml' next to the MCAP.
    - 0 found  -> None
    - 1 found  -> that file
    - >1 found -> error (ambiguous)
    """
    folder = mcap_path.parent

    # CHANGED: only consider .yaml (not .yml)
    yamls = sorted(folder.glob("*.yaml"))
    export_yamls = [p for p in yamls if p.stem.lower().endswith("_export")]

    if len(export_yamls) == 0:
        return None
    if len(export_yamls) == 1:
        return export_yamls[0]

    raise RuntimeError(
        "ambiguous: multiple *_export.yaml files found next to MCAP. "
        f"Found: {[p.name for p in export_yamls]}"
    )


def _yaml_get(y: Any, path: list[str]) -> Any:
    cur = y
    for k in path:
        if not isinstance(cur, dict):
            return None
        cur = cur.get(k)
    return cur


def _to_float(v: Any) -> Optional[float]:
    if v is None:
        return None
    if isinstance(v, bool):
        # avoid True -> 1.0 surprises
        return None
    if isinstance(v, (int, float)):
        return float(v)
    if isinstance(v, str):
        s = v.strip()
        if not s:
            return None
        try:
            return float(s)
        except ValueError:
            return None
    return None


def _to_bool(v: Any) -> Optional[bool]:
    if v is None:
        return None
    if isinstance(v, bool):
        return v
    if isinstance(v, int) and v in (0, 1):
        return bool(v)
    if isinstance(v, str):
        s = v.strip().lower()
        if s in ("true", "1", "yes", "y", "on"):
            return True
        if s in ("false", "0", "no", "n", "off"):
            return False
    return None


def _to_str(v: Any) -> Optional[str]:
    if v is None:
        return None
    if isinstance(v, str):
        s = v.strip()
        return s if s != "" else None
    if isinstance(v, (int, float, bool)):
        return str(v)
    return None


def _to_str_or_number_as_str(v: Any) -> Optional[str]:
    if v is None:
        return None
    if isinstance(v, str):
        return v
    if isinstance(v, (int, float)):
        return str(v)
    return None


class PluginImpl(BasePlugin):
    def run(self, data: str) -> str:
        self.report_progress(0.01, "Starting import")

        self.wait_while_paused()
        if self.should_stop():
            return self._json_result(status="stopped", message="stopped")

        try:
            payload = json.loads(data) if data else {}
            if payload is None:
                payload = {}
            if not isinstance(payload, dict):
                return self._json_result(status="error", message="invalid json payload: expected an object")
        except Exception as e:
            return self._json_result(status="error", message=f"invalid json payload: {e}")

        self.report_progress(0.10, "Validated payload")

        mcap_path_raw = (payload.get("mcap_path") or "").strip()
        if not mcap_path_raw:
            return self._json_result(status="error", message="missing 'mcap_path'")

        mcap_path = Path(mcap_path_raw)
        if not mcap_path.exists():
            return self._json_result(status="error", message=f"mcap_path does not exist: {mcap_path}")
        if mcap_path.suffix.lower() != ".mcap":
            return self._json_result(status="error", message=f"mcap_path must point to a .mcap file: {mcap_path}")

        self.report_progress(0.18, "Looking for export YAML")

        try:
            export_yaml_path = _find_export_yaml_next_to_mcap(mcap_path)
        except Exception as e:
            return self._json_result(status="error", message=str(e))

        if export_yaml_path is None:
            # no progress change; still pending
            return self._json_result(
                status="pending",
                message="no *_export.yaml found yet next to mcap",
                summary={"mcap_path": str(mcap_path), "export_yaml_path": None},
            )

        self.report_progress(0.30, "Parsing YAML")

        try:
            parsed = self._parse_yaml(export_yaml_path)
        except Exception as e:
            return self._json_result(status="error", message=str(e))

        if parsed is None:
            return self._json_result(status="stopped", message="stopped")

        self.report_progress(0.45, "Resolving entry in backend")

        backend_base_url = (payload.get("backend_base_url") or "http://127.0.0.1:8080").rstrip("/")
        txid = int(payload.get("txid") or 0)

        entry_path = str(mcap_path)
        path_part = quote(entry_path.lstrip("/"), safe="/")
        url = f"{backend_base_url}/paths/{path_part}?txid={txid}"

        try:
            entry = self._http_get_json(url)
        except Exception as e:
            return self._json_result(status="error", message=f"failed to fetch entry by path: {e}")

        entry_id = entry.get("id")
        if not isinstance(entry_id, int):
            return self._json_result(status="error", message="backend response missing numeric entry.id")

        self.report_progress(0.60, "Mapping metadata")

        md = self._map_export_yaml_to_metadata_web(parsed)

        self.report_progress(0.75, "Updating metadata")

        try:
            self._http_put_json(f"{backend_base_url}/entries/{entry_id}/metadata/tx/{txid}", md)
        except Exception as e:
            return self._json_result(status="error", message=f"failed to update metadata: {e}")

        tags = self._extract_tags(parsed)
        if tags:
            for i, tag in enumerate(tags, start=1):
                self.wait_while_paused()
                if self.should_stop():
                    return self._json_result(status="stopped", message="stopped")

                # 75%..95% reserved for tags
                frac = i / max(1, len(tags))
                self.report_progress(0.75 + 0.20 * frac, f"Adding tag {i}/{len(tags)}")

                try:
                    self._http_put_text(
                        f"{backend_base_url}/entries/{entry_id}/tags/tx/{txid}",
                        tag,
                    )
                except Exception as e:
                    return self._json_result(status="error", message=f"failed to add tag '{tag}': {e}")

        self.report_progress(1.0, "Done")

        summary = {
            "mcap_path": str(mcap_path),
            "export_yaml_path": str(export_yaml_path),
            "entry_path": entry_path,
            "entry_id": entry_id,
            "tags_added": len(tags),
        }
        return self._json_result(status="ok", message="ok", summary=summary)

    def _map_export_yaml_to_metadata_web(self, y: Any) -> dict[str, Any]:
        # Export YAML is flat (top-level keys). Cast everything defensively.
        if not isinstance(y, dict):
            raise RuntimeError("export yaml must be a mapping/object at top-level")

        return {
            "time_machine": _to_float(y.get("time_machine")),
            "platform_name": _to_str(y.get("platform_name")),
            "platform_image_link": _to_str(y.get("platform_image_link")),
            "scenario_name": _to_str(y.get("scenario_name")),
            "scenario_creation_time": _to_str(y.get("scenario_creation_time")),
            "scenario_description": _to_str(y.get("scenario_description")),
            "sequence_duration": _to_float(y.get("sequence_duration")),
            "sequence_distance": _to_float(y.get("sequence_distance")),
            "sequence_lat_starting_point_deg": _to_float(y.get("sequence_lat_starting_point_deg")),
            "sequence_lon_starting_point_deg": _to_float(y.get("sequence_lon_starting_point_deg")),
            "weather_cloudiness": _to_str(y.get("weather_cloudiness")),
            "weather_precipitation": _to_str(y.get("weather_precipitation")),
            "weather_precipitation_deposits": _to_str(y.get("weather_precipitation_deposits")),
            "weather_wind_intensity": _to_str(y.get("weather_wind_intensity")),
            "weather_road_humidity": _to_str(y.get("weather_road_humidity")),
            "weather_fog": _to_bool(y.get("weather_fog")),
            "weather_snow": _to_bool(y.get("weather_snow")),
            "topics": None,
        }

    def _extract_tags(self, y: Any) -> list[str]:
        if not isinstance(y, dict):
            return []
        v = y.get("tags")
        if not isinstance(v, list):
            return []

        out: list[str] = []
        for item in v:
            s = _to_str(item)
            if s:
                out.append(s)
        return out

    def _http_get_json(self, url: str) -> dict[str, Any]:
        import urllib.request

        req = urllib.request.Request(url, method="GET")
        with urllib.request.urlopen(req, timeout=10) as resp:
            raw = resp.read().decode("utf-8", errors="replace")
        val = json.loads(raw)
        if not isinstance(val, dict):
            raise RuntimeError("expected JSON object")
        return val

    def _http_put_json(self, url: str, obj: dict[str, Any]) -> None:
        import urllib.request

        data = json.dumps(obj).encode("utf-8")
        req = urllib.request.Request(
            url,
            data=data,
            method="PUT",
            headers={"Content-Type": "application/json"},
        )
        with urllib.request.urlopen(req, timeout=10) as resp:
            _ = resp.read()

    def _http_put_text(self, url: str, text: str) -> None:
        import urllib.request

        data = text.encode("utf-8")
        req = urllib.request.Request(
            url,
            data=data,
            method="PUT",
            headers={"Content-Type": "text/plain; charset=utf-8"},
        )
        with urllib.request.urlopen(req, timeout=10) as resp:
            _ = resp.read()

    def _write_import_yaml(self, import_path: Path, parsed: Any, *, overwrite: bool) -> None:
        self.wait_while_paused()
        if self.should_stop():
            raise RuntimeError("stopped")

        try:
            import yaml
        except Exception as e:
            raise RuntimeError(
                "Python dependency missing: cannot import 'yaml' (PyYAML). "
                f"Details: {e}"
            )

        if import_path.exists() and not overwrite:
            raise RuntimeError(
                f"import output already exists: {import_path} (set overwrite_import=true to overwrite)"
            )

        # Normalize by dumping the parsed structure again
        text = yaml.safe_dump(
            parsed,
            sort_keys=False,
            allow_unicode=True,
        )
        import_path.write_text(text, encoding="utf-8")

    def _parse_yaml(self, metadata_path: Path) -> Optional[Any]:
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

        text = metadata_path.read_text(encoding="utf-8", errors="replace")

        self.wait_while_paused()
        if self.should_stop():
            return None

        try:
            return yaml.safe_load(text)
        except Exception as e:
            raise RuntimeError(f"failed to parse yaml: {e}")

    def _json_result(
        self,
        *,
        status: str,
        message: str,
        summary: Optional[dict[str, Any]] = None,
        metadata: Optional[Any] = None,
    ) -> str:
        out: dict[str, Any] = {"status": status, "message": message}
        if summary is not None:
            out["summary"] = summary
        if metadata is not None:
            out["metadata"] = metadata
        return json.dumps(out, ensure_ascii=False)
