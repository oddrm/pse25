# plugin.py
PLUGIN_NAME = "example_plugin"
PLUGIN_DESCRIPTION = "My first plugin written in Python"
PLUGIN_TRIGGER = "manual"

from plugin_base import BasePlugin


class PluginImpl(BasePlugin):
    def step(self, data: str) -> None:
        # plugin Implementierung
        pass