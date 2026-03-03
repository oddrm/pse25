# Was macht das Plugin?
PLUGIN_NAME = "foo"
PLUGIN_DESCRIPTION = "My first plugin written in Python"
# PLUGIN_TRIGGER = "on_schedule: */10 * * * * *"
PLUGIN_TRIGGER = "manual"
STOPPED = "stopped"
# "on_entry_create"
# "on_entry_update"
# "on_entry_delete"
from plugin_base import BasePlugin, TICK_SECONDS
import time
import logging

logger = logging.getLogger(__name__)


class PluginImpl(BasePlugin):
    def run(self, data: str) -> str:
        logger.info("Plugin 'foo' run called with data=%s", data)
        # simple behavior: immediately stop
        logger.debug("Plugin 'foo' returning STOPPED")
        return STOPPED
