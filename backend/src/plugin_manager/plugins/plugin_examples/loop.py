# Was macht das Plugin?
PLUGIN_NAME = "loop"
PLUGIN_DESCRIPTION = "My first plugin written in Python"
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
        logger.info("Plugin 'loop' run started with data=%s", data)
        x = 100
        while not self.should_stop():
            x = x // 2 if x % 2 == 0 else 3 * x + 1
            logger.debug("Plugin 'loop' iteration x=%d", x)
            logger.info(f"PROGRESS:{(x % 100) / 100:.2f}")
            # allow pause/resume
            self.wait_while_paused()
            time.sleep(TICK_SECONDS * 10)

        logger.info("Plugin 'loop' stopping, final x=%d", x)
        return STOPPED
