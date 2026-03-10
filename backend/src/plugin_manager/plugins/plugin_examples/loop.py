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

# Eigener Logger für dieses Plugin.
# Die Log-Ausgaben werden vom Runner abgefangen und an Rust weitergereicht.
logger = logging.getLogger(__name__)


class PluginImpl(BasePlugin):
    def run(self, data: str) -> str:
        """
        Beispiel für ein langlaufendes Plugin.

        Das Plugin:
        - schreibt Logs
        - berechnet in einer Schleife neue Werte
        - meldet künstlichen Fortschritt
        - reagiert auf Pause/Stop
        """
        logger.info("Plugin 'loop' run started with data=%s", data)

        # Startwert für die Beispielberechnung.
        x = 100

        # Schleife läuft, bis von außen stop() angefordert wird.
        while not self.should_stop():
            # Beispielrechnung:
            # Wenn x gerade ist -> halbiere x
            # sonst -> 3*x + 1
            # Das erinnert an die Collatz-Folge und dient hier nur als Demo.
            x = x // 2 if x % 2 == 0 else 3 * x + 1

            logger.debug("Plugin 'loop' iteration x=%d", x)

            # Fortschritt als Log ausgeben.
            # Der Runner erkennt Logs mit "PROGRESS:" speziell
            # und leitet sie als Progress-Event weiter.
            logger.info(f"PROGRESS:{(x % 100) / 100:.2f}")

            # Unterstützt Pause/Resume:
            # Wenn pausiert, blockiert das Plugin hier kooperativ.
            self.wait_while_paused()

            # Kleine Pause, damit die Schleife nicht ungebremst läuft.
            time.sleep(TICK_SECONDS * 10)

        logger.info("Plugin 'loop' stopping, final x=%d", x)
        return STOPPED
