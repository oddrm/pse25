PLUGIN_NAME = "compress"
PLUGIN_DESCRIPTION = "Compress selected entry. Only run on a single plugin"
PLUGIN_TRIGGER = "manual"
STOPPED = "stopped"
from plugin_base import BasePlugin
import json
import logging
import subprocess


logger = logging.getLogger(__name__)


class PluginImpl(BasePlugin):
    def run(self, data) -> str:
        logger.info("Plugin 'compress' run called with data=%s", data)
        # Normalize data: plugins sometimes receive a JSON string or a raw path
        if data is None:
            logger.error("Plugin 'compress' missing data")
            return STOPPED

        parsed = None
        if isinstance(data, str):
            # Try JSON first, fall back to treating the string as the entry path
            try:
                parsed = json.loads(data)
            except Exception:
                parsed = {"entry_path": data}
        elif isinstance(data, dict):
            parsed = data
        else:
            logger.error(
                "Plugin 'compress' received unsupported data type: %s",
                type(data),
            )
            return STOPPED

        if "entry_path" not in parsed or parsed["entry_path"] is None:
            logger.error("Plugin 'compress' missing 'entry_path' in data")
            return STOPPED

        output_path = str(parsed["entry_path"]).replace(".mcap", ".compressed.mcap")
        cmd = f"mcap compress {parsed['entry_path']} -o {output_path}"
        logger.info("Plugin 'compress' executing command: %s", cmd)
        # execute command directly
        try:
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
            logger.error(
                "Plugin 'compress' command failed with return code %s", e.returncode
            )
            logger.error("Plugin 'compress' command output: %s", e.output.decode())
            logger.error(
                "Plugin 'compress' command error output: %s", e.stderr.decode()
            )
        return STOPPED
