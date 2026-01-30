# plugin.py
PLUGIN_NAME = "example_plugin"
PLUGIN_DESCRIPTION = "My first plugin written in Python"
PLUGIN_TRIGGER = "manual"

from plugin_base import BasePlugin


class PluginImpl(BasePlugin):
    def step(self, data: str) -> None:
        # plugin Implementierung
        pass
    
    def run(self, data: str) -> str:
        # Logs öffnen, Ressourcen initialisieren, Status setzen, ...
        try:
            return super().run(data)  # Standard-Worker-Loop (step() in Schleife)
        finally:
            # Dateien schließen, temporäre Ressourcen freigeben, ...
            pass

       