# plugin_base.py
from __future__ import annotations

from pathlib import Path
import threading


class BasePlugin:
    """
    Gemeinsame Basis für alle Plugins:
    - kooperatives Stoppen per Event
    - Pause/Resume per Event
    - run() wird vom Plugin selbst implementiert (kein step()-Loop mehr)
    """

    TICK_SECONDS = 0.1  # wie oft wir beim Warten "ticken", damit Stop schnell reagiert

    def __init__(self, path: str):
        self.path = Path(path)
        self._stop_event = threading.Event()
        self._pause_event = threading.Event()

    def run(self, data: str) -> str:
        """
        Muss vom Plugin überschrieben werden.
        Implementierung sollte stop()/pause()/resume() respektieren.
        """
        raise NotImplementedError("Plugin must implement run(data)")

    # Helper: im run()-Code nutzbar
    def should_stop(self) -> bool:
        return self._stop_event.is_set()

    def wait_while_paused(self) -> None:
        """
        Blockt kooperativ während Pause aktiv ist, reagiert aber schnell auf stop().
        """
        while self._pause_event.is_set() and not self._stop_event.is_set():
            self._stop_event.wait(self.TICK_SECONDS)

    def pause(self) -> str:
        self._pause_event.set()
        return "paused"

    def resume(self) -> str:
        self._pause_event.clear()
        return "resumed"

    def stop(self) -> str:
        self._stop_event.set()
        return "stopping"