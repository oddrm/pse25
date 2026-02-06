# Was macht das Plugin?
PLUGIN_NAME = "example_plugin"
PLUGIN_DESCRIPTION = "My first plugin written in Python"
PLUGIN_TRIGGER = "manual"
STOPPED = "stopped"
# "on_entry_create"
# "on_entry_update"
# "on_entry_delete"

from plugin_base import BasePlugin, TICK_SECONDS
import time


class PluginImpl(BasePlugin):
    def run(self, data: str) -> str:
        # Langläufer bis stop() kommt
        while not self.should_stop():
            self.wait_while_paused()
            if self.should_stop():
                break

            # Plugin

            time.sleep(TICK_SECONDS)

        return STOPPED
