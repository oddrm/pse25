from __future__ import annotations

import json
import os
import time
import urllib.request
from pathlib import Path

from plugin_base import BasePlugin

# Plugin-Metadaten
PLUGIN_NAME = "import_plugin"
PLUGIN_DESCRIPTION = "Imports a remote file via HTTP(S) by downloading it into a local folder."
PLUGIN_TRIGGER = "manual"


class PluginImpl(BasePlugin):
    """
    Import-Plugin: lädt eine Datei via HTTP(S) herunter und speichert sie in einem lokalen Ordner.

    Erwartetes Input-Format für run(data):
      - data ist ein JSON-String (oder leer).
      - Beispiel:
        {
          "url": "https://example.invalid/file.mcap",
          "dest_dir": "C:/path/to/watch/dir",
          "filename": "optional_override.mcap",
          "timeout_seconds": 30
        }

    Felder:
      - url (pflicht): Download-URL (http/https).
      - dest_dir (optional): Zielordner. Wenn nicht gesetzt, wird IMPORT_PLUGIN_DEST_DIR (env) genutzt.
      - filename (optional): Dateiname. Falls nicht gesetzt, wird er aus der URL oder einem Timestamp abgeleitet.
      - timeout_seconds (optional): Timeout für HTTP-Request in Sekunden (Default 30).

    Pause/Stop:
      - Dieses Plugin unterstützt "cooperative" Pause/Stop über BasePlugin:
        * wait_while_paused(): blockiert solange pausiert
        * should_stop(): signalisiert Abbruch
      - Dadurch kann ein Download sauber unterbrochen werden (inkl. Aufräumen der .partial-Datei).

      Docker:
      docker run --rm \
          -e IMPORT_PLUGIN_DEST_DIR=/data/imports \
          -v /host/path/imports:/data/imports \
          <IMAGE_NAME>
      docker-compose.yml
      services:
          backend:
            image: <IMAGE_NAME>
            environment:
              IMPORT_PLUGIN_DEST_DIR: /data/imports
            volumes:
              - ./imports:/data/imports
    """

    def run(self, data: str) -> str:
        # Payload parsen (data kann leer sein → {}).
        try:
            payload = json.loads(data) if data else {}
        except Exception as e:
            # Fehler zurückgegeben
            return f"error: invalid json payload: {e}"

        # url extrahieren und prüfen
        url = (payload.get("url") or "").strip()
        if not url:
            return "error: missing 'url'"

        # dest_dir entweder aus Payload oder ENV-Variable beziehen
        dest_dir_raw = (payload.get("dest_dir") or os.environ.get("IMPORT_PLUGIN_DEST_DIR") or "").strip()
        if not dest_dir_raw:
            return "error: missing 'dest_dir' (or env IMPORT_PLUGIN_DEST_DIR)"

        # timeout_seconds robust auf float parsen (Fallback: 30s)
        timeout_seconds = payload.get("timeout_seconds", 30)
        try:
            timeout_seconds = float(timeout_seconds)
        except Exception:
            timeout_seconds = 30.0

        # Zielordner sicherstellen
        dest_dir = Path(dest_dir_raw)
        dest_dir.mkdir(parents=True, exist_ok=True)

        # filename bestimmen: Payload > URL-Segment > Timestamp
        filename = (payload.get("filename") or "").strip()
        if not filename:
            # fallback: last URL segment or a timestamp
            last = url.split("/")[-1].split("?")[0].strip()
            filename = last if last else f"import_{int(time.time())}.bin"

        # Pfade: finale Datei + temporäre Partial-Datei
        final_path = dest_dir / filename
        tmp_path = final_path.with_suffix(final_path.suffix + ".partial")

        # Safety: nicht überschreiben (verhindert unbeabsichtigten Datenverlust)
        if final_path.exists():
            return f"error: file already exists: {final_path}"

        # Vor Netzwerkstart: Pause/Stop kooperativ respektieren
        self.wait_while_paused()
        if self.should_stop():
            return "stopped"

        # HTTP-Request vorbereiten (User-Agent ist hilfreich für Logs/Serverregeln)
        req = urllib.request.Request(
            url,
            headers={
                "User-Agent": "rosbag-manager-import-plugin/1.0",
            },
            method="GET",
        )

        try:
            # Download starten (mit Timeout)
            with urllib.request.urlopen(req, timeout=timeout_seconds) as resp:
                # Stream-Download in Chunks, damit Pause/Stop "zwischenrein" greifen kann
                with open(tmp_path, "wb") as f:
                    while True:
                        # Pause/Stop in der Schleife prüfen: so bleibt der Download steuerbar
                        self.wait_while_paused()
                        if self.should_stop():
                            # Aufräumen: partial Datei entfernen, damit kein "halber" Import liegen bleibt
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

            # Nach erfolgreichem Download: partial "atomar-ish" auf final umbenennen
            tmp_path.replace(final_path)
            return f"ok: downloaded to {final_path}"

        except Exception as e:
            # Fehlerfall: partial entfernen, dann Fehlermeldung zurückgeben
            try:
                tmp_path.unlink(missing_ok=True)
            except Exception:
                pass
            return f"error: download failed: {e}"