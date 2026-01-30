# plugin.py
PLUGIN_NAME = "example_plugin"
PLUGIN_DESCRIPTION = "My first plugin written in Python"
PLUGIN_TRIGGER = "manual"

from plugin_base import BasePlugin
import time


class PluginImpl(BasePlugin):
    def run(self, data: str) -> str:
        # Langläufer bis stop() kommt
        while not self.should_stop():
            self.wait_while_paused()
            if self.should_stop():
                break

            # Plugin
            
            time.sleep(self.TICK_SECONDS)

        return "stopped"
