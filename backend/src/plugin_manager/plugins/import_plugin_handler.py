from __future__ import annotations

import json
import os
import time
import urllib.request
from pathlib import Path

from plugin_base import BasePlugin

PLUGIN_NAME = "import_plugin"
PLUGIN_DESCRIPTION = "Imports a remote file via HTTP(S) by downloading it into a local folder."
PLUGIN_TRIGGER = "manual"

class PluginImpl(BasePlugin):
    """
    Data format for run(data) is JSON (string), e.g.:
      {
        "url": "https://example.invalid/file.mcap",
        "dest_dir": "C:/path/to/watch/dir",
        "filename": "optional_override.mcap",
        "timeout_seconds": 30
      }

    If dest_dir is omitted, it falls back to env IMPORT_PLUGIN_DEST_DIR.
    """

    def run(self, data: str) -> str:
        try:
            payload = json.loads(data) if data else {}
        except Exception as e:
            return f"error: invalid json payload: {e}"

        url = (payload.get("url") or "").strip()
        if not url:
            return "error: missing 'url'"

        dest_dir_raw = (payload.get("dest_dir") or os.environ.get("IMPORT_PLUGIN_DEST_DIR") or "").strip()
        if not dest_dir_raw:
            return "error: missing 'dest_dir' (or env IMPORT_PLUGIN_DEST_DIR)"

        timeout_seconds = payload.get("timeout_seconds", 30)
        try:
            timeout_seconds = float(timeout_seconds)
        except Exception:
            timeout_seconds = 30.0

        dest_dir = Path(dest_dir_raw)
        dest_dir.mkdir(parents=True, exist_ok=True)

        filename = (payload.get("filename") or "").strip()
        if not filename:
            # fallback: last URL segment or a timestamp
            last = url.split("/")[-1].split("?")[0].strip()
            filename = last if last else f"import_{int(time.time())}.bin"

        final_path = dest_dir / filename
        tmp_path = final_path.with_suffix(final_path.suffix + ".partial")

        # If already exists, don't overwrite by default (safer for demo).
        if final_path.exists():
            return f"error: file already exists: {final_path}"

        # cooperative pause/stop before starting network
        self.wait_while_paused()
        if self.should_stop():
            return "stopped"

        req = urllib.request.Request(
            url,
            headers={
                "User-Agent": "rosbag-manager-import-plugin/1.0",
            },
            method="GET",
        )

        try:
            with urllib.request.urlopen(req, timeout=timeout_seconds) as resp:
                # Stream download in chunks so stop/pause can interrupt
                with open(tmp_path, "wb") as f:
                    while True:
                        self.wait_while_paused()
                        if self.should_stop():
                            try:
                                f.flush()
                            finally:
                                f.close()
                            try:
                                tmp_path.unlink(missing_ok=True)
                            except Exception:
                                pass
                            return "stopped"

                        chunk = resp.read(1024 * 256)  # 256 KiB
                        if not chunk:
                            break
                        f.write(chunk)

            # atomic-ish move into place
            tmp_path.replace(final_path)
            return f"ok: downloaded to {final_path}"

        except Exception as e:
            try:
                tmp_path.unlink(missing_ok=True)
            except Exception:
                pass
            return f"error: download failed: {e}"
