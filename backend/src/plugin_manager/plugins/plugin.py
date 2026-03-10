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
        """
        Haupteinstiegspunkt des Plugins.

        Diese Methode wird vom Python-Runner in einem Worker-Thread gestartet.
        Das Plugin läuft hier als kooperative Schleife:
        - es prüft regelmäßig, ob ein Stop angefordert wurde
        - es blockiert sauber bei Pause
        - es führt dazwischen seine eigentliche Arbeit aus

        `data` enthält optionale Zusatzdaten, die z. B. vom Frontend
        oder vom Rust-Manager übergeben wurden.
        """

        # Langläufer-Schleife:
        # Das Plugin arbeitet so lange, bis von außen stop() angefordert wird.
        while not self.should_stop():
            # Falls das Plugin pausiert wurde, wartet es hier kooperativ.
            # Die Methode kehrt erst zurück, wenn Resume kommt oder Stop gesetzt ist.
            self.wait_while_paused()

            # Nach dem Warten nochmal Stop prüfen, damit das Plugin
            # schnell und sauber beendet werden kann.
            if self.should_stop():
                break

            # -----------------------------------------
            # HIER kommt die eigentliche Plugin-Logik hin
            # -----------------------------------------
            # Beispiel:
            # - Dateien verarbeiten
            # - Daten transformieren
            # - externe Systeme ansprechen
            # - Status berechnen
            #
            # Aktuell macht das Beispiel absichtlich "nichts Sichtbares"
            # und schläft nur kurz, um eine laufende Arbeit zu simulieren.

            time.sleep(TICK_SECONDS)

        # Rückgabewert für den Runner / Manager:
        # signalisiert, dass das Plugin regulär beendet wurde.
        return STOPPED
