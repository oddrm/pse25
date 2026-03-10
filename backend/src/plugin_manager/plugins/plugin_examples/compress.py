# Metadaten für den Plugin-Manager.
PLUGIN_NAME = "compress"
PLUGIN_DESCRIPTION = "Compress selected entry. Only run on a single plugin"
PLUGIN_TRIGGER = "manual"

# Standard-Rückgabewert bei regulärem Ende.
STOPPED = "stopped"

from plugin_base import BasePlugin
import json
import logging
import subprocess


# Logger dieses Plugins.
logger = logging.getLogger(__name__)


class PluginImpl(BasePlugin):
    def run(self, data) -> str:
        """
        Komprimiert eine ausgewählte MCAP-Datei per externem Kommando.

        Erwartet in `data` einen JSON-String mit mindestens:
        {
          "entry_path": "/pfad/zur/datei.mcap"
        }

        Ablauf:
        1. Eingabedaten prüfen
        2. Zielpfad erzeugen
        3. `mcap compress` ausführen
        4. stdout/stderr loggen
        """
        logger.info("Plugin 'compress' run called with data=%s", data)

        # Ohne Daten kann das Plugin nichts tun.
        if data is None:
            logger.error("Plugin 'compress' missing data")
            return STOPPED

        parsed = None
        try:
            # Eingabe als JSON interpretieren.
            parsed = json.loads(data)
        except Exception:
            logger.error(
                "Plugin 'metadata_yaml_export' failed to parse input data as JSON: %s",
                data,
            )
            return STOPPED

        # Das Plugin benötigt einen `entry_path`.
        if "entry_path" not in parsed or parsed["entry_path"] is None:
            logger.error("Plugin 'compress' missing 'entry_path' in data")
            return STOPPED

        # Zielname erzeugen:
        # aus foo.mcap wird foo.compressed.mcap
        output_path = str(parsed["entry_path"]).replace(".mcap", ".compressed.mcap")

        # Shell-Kommando zusammensetzen.
        cmd = f"mcap compress {parsed['entry_path']} -o {output_path}"
        logger.info("Plugin 'compress' executing command: %s", cmd)

        try:
            # Externes Kommando ausführen.
            # stdout/stderr werden eingesammelt und später geloggt.
            result = subprocess.run(
                cmd,
                shell=True,
                check=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

            logger.info("Plugin 'compress' command output: %s", result.stdout.decode())
            logger.info(
                "Plugin 'compress' command error output: %s", result.stderr.decode()
            )
        except subprocess.CalledProcessError as e:
            # Fehlerfall:
            # Rückgabecode, stdout und stderr werden geloggt.
            logger.error(
                "Plugin 'compress' command failed with return code %s", e.returncode
            )
            logger.error("Plugin 'compress' command output: %s", e.output.decode())
            logger.error(
                "Plugin 'compress' command error output: %s", e.stderr.decode()
            )

        return STOPPED
