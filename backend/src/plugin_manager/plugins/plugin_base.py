# plugin_base.py
from __future__ import annotations

from pathlib import Path
import threading

# -------------------- constants --------------------
# Wie oft beim Warten auf Stop/Pause intern geprüft wird.
# Ein kleiner Wert macht Stop/Resume reaktionsschneller.
TICK_SECONDS = 0.1

# Fehlermeldung, falls ein Plugin run() nicht überschreibt.
ERR_NOT_IMPLEMENTED = "Plugin must implement run(data)"

# Standard-Rückgabewerte für Steuerkommandos.
RESULT_PAUSED = "paused"
RESULT_RESUMED = "resumed"
RESULT_STOPPING = "stopping"

# Hinweise zur Ablage:
# - plugins_dir/config enthält Konfiguration
# - Plugins liegen unter plugins_dir
# - YAML aus src/plugin_manager/plugins/config muss nach plugins_dir/config kopiert werden

class BasePlugin:
    """
    Gemeinsame Basisklasse für alle Python-Plugins.

    Diese Klasse kapselt die Standard-Steuerung eines Plugins:
    - kooperatives Stoppen über ein Thread-Event
    - Pause/Resume über ein weiteres Event
    - Zugriff auf Pfad und Zusatzdaten (`data`)

    Wichtig:
    Die eigentliche Fachlogik implementiert das konkrete Plugin
    selbst in `run(data)`.
    """

    def __init__(self, path: str, data: str = ""):
        # Pfad, unter dem das Plugin arbeitet oder auf das es sich bezieht.
        # Je nach Trigger kann das z. B. eine Datei, ein Verzeichnis oder
        # einfach ein Kontextpfad sein.
        self.path = Path(path)

        # Beliebige Zusatzdaten, die vom Manager/Frontend mitgegeben wurden.
        # Das kann JSON als String sein oder einfacher Freitext.
        self.data = data

        # Stop-Event:
        # Sobald gesetzt, soll das Plugin seine Arbeit kontrolliert beenden.
        self._stop_event = threading.Event()

        # Pause-Event:
        # Wenn gesetzt, soll das Plugin an geeigneten Stellen warten,
        # bis resume() aufgerufen wird.
        self._pause_event = threading.Event()

    def run(self, data: str) -> str:
        """
        Muss von einem konkreten Plugin überschrieben werden.

        Erwartung an Implementierungen:
        - sollten regelmäßig `should_stop()` prüfen
        - sollten `wait_while_paused()` respektieren
        - sollten am Ende einen sinnvollen Status/String zurückgeben
        """
        raise NotImplementedError(ERR_NOT_IMPLEMENTED)

    def should_stop(self) -> bool:
        """
        Gibt zurück, ob ein Stop angefordert wurde.
        """
        return self._stop_event.is_set()

    def wait_while_paused(self) -> None:
        """
        Blockiert kooperativ, solange Pause aktiv ist.

        Dabei wird nicht "hart" endlos geschlafen, sondern in kleinen Intervallen
        gewartet. So kann das Plugin trotzdem schnell auf stop() reagieren.
        """
        while self._pause_event.is_set() and not self._stop_event.is_set():
            self._stop_event.wait(TICK_SECONDS)

    def pause(self) -> str:
        """
        Aktiviert den Pausenmodus.
        Das Plugin hält dann an geeigneten Stellen in `wait_while_paused()`.
        """
        self._pause_event.set()
        return RESULT_PAUSED

    def resume(self) -> str:
        """
        Hebt den Pausenmodus wieder auf.
        """
        self._pause_event.clear()
        return RESULT_RESUMED

    def stop(self) -> str:
        """
        Signalisiert dem Plugin, dass es sich sauber beenden soll.
        """
        self._stop_event.set()
        return RESULT_STOPPING