PLUGIN_NAME = "mcap_info_compressed"
PLUGIN_DESCRIPTION = "Create a gzip-compressed sidecar file containing the plaintext output of `mcap info` (stored as bytes) next to the .mcap"
PLUGIN_TRIGGER = "on_entry_create"
STOPPED = "stopped"

from plugin_base import BasePlugin
import gzip
import json
import logging
import subprocess
from pathlib import Path
from typing import Any

logger = logging.getLogger(__name__)


class PluginImpl(BasePlugin):
    """
    Plugin-Syntax wie in den plugin_examples:
    - Metadata Konstanten am Anfang
    - Implementierung in class PluginImpl(BasePlugin)
    - run(data: str) -> str liefert am Ende STOPPED

    Sidecar-Datei (im selben Ordner wie die MCAP):
      <datei>.mcap.info.txt.gz
    Beispiel:
      demo.mcap -> demo.mcap.info.txt.gz
    """

    def _extract_mcap_path(self, data: str) -> Path:
        data = (data or "").strip()
        if not data:
            raise ValueError("empty data: expected JSON with 'mcap_path' or a file path string")

        # bevorzugt JSON payload: {"mcap_path":"..."}
        try:
            obj: Any = json.loads(data)
            if isinstance(obj, dict):
                p = obj.get("mcap_path")
                if isinstance(p, str) and p.strip():
                    return Path(p)
        except json.JSONDecodeError:
            pass

        # fallback: data ist direkt der Pfad
        return Path(data)

    def _output_path_next_to_mcap(self, mcap_path: Path) -> Path:
        return mcap_path.with_name(mcap_path.name + ".info.txt.gz")

    def run(self, data: str) -> str:
        try:
            mcap_path = self._extract_mcap_path(data)
        except Exception:
            logger.exception("failed to parse input data")
            return STOPPED

        if not mcap_path.exists():
            logger.error("mcap not found: %s", mcap_path)
            return STOPPED

        out_path = self._output_path_next_to_mcap(mcap_path)

        self.wait_while_paused()
        if self.should_stop():
            logger.info("%s stopping on request", PLUGIN_NAME)
            return STOPPED

        try:
            completed = subprocess.run(
                ["mcap", "info", str(mcap_path)],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
            )
        except FileNotFoundError:
            logger.error("'mcap' CLI not found in PATH")
            return STOPPED
        except Exception:
            logger.exception("failed to run `mcap info`")
            return STOPPED

        self.wait_while_paused()
        if self.should_stop():
            logger.info("%s stopping on request", PLUGIN_NAME)
            return STOPPED

        if completed.returncode != 0:
            stderr_txt = (completed.stderr or b"").decode("utf-8", errors="replace").strip()
            logger.error("mcap info failed (code=%s): %s", completed.returncode, stderr_txt)
            return STOPPED

        stdout_bytes = completed.stdout or b""

        try:
            out_path.parent.mkdir(parents=True, exist_ok=True)
            with gzip.open(out_path, "wb", compresslevel=6) as f:
                chunk_size = 256 * 1024
                for i in range(0, len(stdout_bytes), chunk_size):
                    self.wait_while_paused()
                    if self.should_stop():
                        logger.info("%s stopping on request", PLUGIN_NAME)
                        return STOPPED
                    f.write(stdout_bytes[i : i + chunk_size])
        except Exception:
            logger.exception("failed to write gzip file: %s", out_path)
            return STOPPED

        logger.info(
            "wrote sidecar next to MCAP: %s (raw_bytes=%d)",
            out_path.name,
            len(stdout_bytes),
        )
        return STOPPED
