# plugin_base.py
from __future__ import annotations

from pathlib import Path
import threading
import time


class BasePlugin:
    """
    Gemeinsame Basis für alle Plugins:
    - kooperatives Stoppen per Event
    - Pause/Resume per Event
    - run() als Worker-Loop, der step() aufruft
    """

    TICK_SECONDS = 0.1  # wie oft der Loop "tickt", wenn nichts zu tun ist

    def __init__(self, path: str):
        self.path = Path(path)
        self._stop_event = threading.Event()
        self._pause_event = threading.Event()

    def run(self, data: str) -> str:
        """
        Default: Langlaufender Worker.
        Ruft step(data) in einer Schleife auf, bis stop() gesetzt wird.
        """
        while not self._stop_event.is_set():
            if self._pause_event.is_set():
                # gibt GIL frei und reagiert trotzdem schnell auf stop()
                self._stop_event.wait(self.TICK_SECONDS)
                continue

            # Ein "Arbeitsschritt"
            self.step(data)

            # Wichtig: yield, damit stop()/pause() schnell greifen
            time.sleep(self.TICK_SECONDS)

        return "stopped"

    def step(self, data: str) -> None:
        """
        Muss von Plugins überschrieben werden (ein kleiner Arbeitsschritt).
        """
        raise NotImplementedError("Plugin must implement step(data)")

    def pause(self) -> str:
        self._pause_event.set()
        return "paused"

    def resume(self) -> str:
        self._pause_event.clear()
        return "resumed"

    def stop(self) -> str:
        self._stop_event.set()
        return "stopping"