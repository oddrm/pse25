from __future__ import annotations

import argparse
import importlib.util
import json
import sys
import threading
import traceback
from pathlib import Path
from typing import Any, Final, Literal, NotRequired, TypedDict
import time
import logging


# Kennzeichnung für direkten Skriptstart.
MAIN__ = "__main__"

EVENT: Final[Literal["event"]] = "event"
PLUGIN_IMPL: Final[Literal["PluginImpl"]] = "PluginImpl"

DATA: Final[Literal["--data"]] = "--data"
ID: Final[Literal["--instance-id"]] = "--instance-id"
PATH: Final[Literal["--plugin-path"]] = "--plugin-path"

TRACE: Final[Literal["trace"]] = "trace"
ERROR: Final[Literal["error"]] = "error"
RESULT: Final[Literal["result"]] = "result"
OK: Final[Literal["ok"]] = "ok"

REQUEST_ID: Final[Literal["request_id"]] = "request_id"
INSTANCE_ID: Final[Literal["instance_id"]] = "instance_id"

CMD = "cmd"
CMD_START = "start"
CMD_PAUSE = "pause"
CMD_RESUME = "resume"
CMD_STOP = "stop"
CMD_STATUS = "status"

EVENT_INIT_ERROR = "init_error"
EVENT_EXITED = "exited"

UNKNOWN_ERROR = "unknown_error"
ERR_WRONG_INSTANCE_ID = "wrong_instance_id"
ERR_UNKNOWN_CMD_PREFIX = "unknown_cmd:"

RESULT_STARTED = "started"

STATUS_RUNNING = "running"
STATUS_STOPPED = "stopped"

# Minimum time (seconds) a plugin process should live to ensure logs are observed
MIN_LIFETIME_SECONDS = 3.0


# Rückgabe Status
def build_status(
    worker_thread: threading.Thread | None, run_done: threading.Event
) -> dict:
    """
    Baut eine Status-Antwort für das `status`-Kommando.

    Der Rust-Manager nutzt diese Rückgabe z. B. für Liveness-Checks.
    """
    return {
        STATUS_RUNNING: worker_thread is not None and worker_thread.is_alive(),
        STATUS_STOPPED: run_done.is_set(),
    }


def emit_exited(instance_id: int, run_result: "RunResult") -> None:
    """
    Sendet das finale 'exited'-Event an den Rust-Manager.

    Dieses Event informiert darüber, ob der Pluginlauf erfolgreich war
    und welches Ergebnis bzw. welcher Fehler am Ende vorlag.
    """
    write_msg(
        {
            INSTANCE_ID: instance_id,
            EVENT: EVENT_EXITED,
            OK: run_result[OK],
            RESULT: run_result.get(RESULT),
            ERROR: run_result.get(ERROR),
            TRACE: run_result.get(TRACE),
        }
    )


def load_plugin(plugin_path: str):
    """
    Lädt das konkrete Plugin-Modul direkt aus einer Datei.

    Wichtig:
    Es wird ein eindeutiger Modulname erzeugt, damit es keine Konflikte
    in `sys.modules` gibt, wenn mehrere Plugins ähnlich heißen oder mehrfach
    geladen werden.
    """
    plugin_file = Path(plugin_path).resolve()
    plugin_dir = str(plugin_file.parent)

    # Plugin-Verzeichnis in den Python-Importpfad aufnehmen.
    if plugin_dir not in sys.path:
        sys.path.insert(0, plugin_dir)

    # Eindeutiger Modulname pro Plugin-Datei.
    unique_name = f"user_plugin_{plugin_file.stem}_{abs(hash(str(plugin_file)))}"

    # Loader für genau diese Datei erzeugen.
    spec = importlib.util.spec_from_file_location(unique_name, str(plugin_file))
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Cannot load plugin from {plugin_path}")

    # Modulobjekt erzeugen und Code ausführen.
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def write_msg(obj: dict) -> None:
    """
    Schreibt eine einzelne JSON-Nachricht auf stdout.

    stdout ist hier der IPC-Kanal zum Rust-Manager.
    Jede Nachricht ist genau eine Zeile.
    """
    sys.stdout.write(json.dumps(obj, ensure_ascii=False) + "\n")
    sys.stdout.flush()


def write_ack(
    instance_id: int,
    request_id: str | None,
    ok: bool,
    result=None,
    error: str | None = None,
    trace: str | None = None,
) -> None:
    """
    Sendet eine standardisierte Antwort auf ein Kommando.

    - Bei Erfolg: `ok = true` und optional `result`
    - Bei Fehler: `ok = false`, plus Fehlertext und optional Stacktrace
    """
    msg = {
        INSTANCE_ID: instance_id,
        REQUEST_ID: request_id,
        OK: ok,
    }
    if ok:
        msg[RESULT] = result
    else:
        msg[ERROR] = error or UNKNOWN_ERROR
        if trace:
            msg[TRACE] = trace
    write_msg(msg)


class RunResult(TypedDict):
    """
    Interne Struktur für das Endergebnis des Worker-Threads.
    """
    instance_id: int
    request_id: str
    ok: bool
    result: NotRequired[Any]
    error: NotRequired[str]
    trace: NotRequired[str]
    event: NotRequired[str]


def main() -> int:
    """
    Einstiegspunkt des Runners.

    Aufgabe des Runners:
    - Plugin-Datei laden
    - Plugin instanziieren
    - Worker-Thread für `plugin.run(...)` starten
    - Kommandos vom Rust-Manager über stdin annehmen
    - Antworten und Events über stdout zurücksenden
    """
    ap = argparse.ArgumentParser()

    # Parameter, die vom Rust-Manager gesetzt werden.
    ap.add_argument(PATH, required=True)
    ap.add_argument(ID, required=True, type=int)
    ap.add_argument(DATA, default="")
    args = ap.parse_args()

    # Eindeutige ID dieser Instanz.
    instance_id = args.instance_id

    try:
        # Plugin-Modul laden.
        module = load_plugin(args.plugin_path)

        # Eigener JSON-Logging-Handler:
        # leitet Python-Logs als strukturierte Nachrichten an Rust weiter.
        class JsonHandler(logging.Handler):
            def emit(self, record: logging.LogRecord) -> None:
                try:
                    # Spezialfall:
                    # Logs mit "PROGRESS:" werden als Fortschritt interpretiert.
                    if record.msg.startswith("PROGRESS:"):
                        progress_str = record.msg[len("PROGRESS:") :].strip()
                        write_msg(
                            {
                                INSTANCE_ID: instance_id,
                                EVENT: "progress",
                                RESULT: {
                                    "progress": float(progress_str),
                                },
                            }
                        )
                        pass

                    # Normales Log-Event an den Manager senden.
                    write_msg(
                        {
                            INSTANCE_ID: instance_id,
                            EVENT: "log",
                            RESULT: {
                                "level": record.levelname,
                                "msg": self.format(record),
                                "logger": record.name,
                            },
                        }
                    )
                except Exception as e:
                    # Logging darf nie selbst den Runner abstürzen lassen.
                    write_msg(
                        {
                            INSTANCE_ID: instance_id,
                            EVENT: "log",
                            RESULT: {
                                "level": "ERROR",
                                "msg": str(e),
                                "logger": record.name,
                            },
                        }
                    )
                    pass

        # Root-Logger neu konfigurieren, damit Logs zentral als JSON laufen.
        root_logger = logging.getLogger()
        root_logger.handlers.clear()
        json_handler = JsonHandler()
        json_handler.setFormatter(logging.Formatter("%(message)s"))
        root_logger.addHandler(json_handler)
        root_logger.setLevel(logging.DEBUG)

        # Plugin-Klasse/Fabrik aus dem Modul holen.
        plugin_impl = getattr(module, PLUGIN_IMPL)

        # Plugin-Instanz erzeugen.
        # Der Runner übergibt hier Pfad und Zusatzdaten.
        plugin = plugin_impl(args.plugin_path, args.data)

        # Optionaler Smoke-Test:
        # Falls das Plugin ein Fortschritts-API hat, kann sofort ein erster Tick gemeldet werden.
        try:
            if hasattr(plugin, "report_progress"):
                plugin.report_progress(0.0, "started")
        except Exception:
            pass

    except Exception as e:
        # Initialisierungsfehler werden als spezielles Event zurückgegeben.
        write_msg(
            {
                INSTANCE_ID: instance_id,
                EVENT: EVENT_INIT_ERROR,
                OK: False,
                ERROR: str(e),
                TRACE: traceback.format_exc(),
            }
        )
        return 1

    # Signalisiert, dass `plugin.run(...)` fertig ist.
    run_done = threading.Event()

    # Sequenzzähler für Request-IDs.
    seq = 0
    request_id = f"{instance_id}-{seq}"
    seq += 1

    # Endergebnis des Pluginlaufs.
    run_result: RunResult = {
        INSTANCE_ID: instance_id,
        REQUEST_ID: request_id,
        OK: True,
    }

    def run_worker():
        """
        Führt das Plugin in einem separaten Worker-Thread aus.

        Der Hauptthread bleibt dadurch frei, um Steuerkommandos
        wie pause/resume/stop/status zu verarbeiten.
        """
        start_time = time.monotonic()
        try:
            # Eigentliche Pluginlogik starten.
            res = plugin.run(args.data)
            run_result[RESULT] = res
        except Exception as exception:
            run_result[OK] = False
            run_result[ERROR] = str(exception)
            run_result[TRACE] = traceback.format_exc()
        finally:
            # Sorgt dafür, dass der Prozess mindestens kurz genug lebt,
            # damit Logs und Abschlussnachrichten noch sauber beim Manager ankommen.
            try:
                elapsed = time.monotonic() - start_time
                if elapsed < MIN_LIFETIME_SECONDS:
                    time.sleep(MIN_LIFETIME_SECONDS - elapsed)
            except Exception:
                pass

            # Pluginlauf ist beendet.
            run_done.set()

    # Referenz auf den Worker-Thread.
    worker_thread: threading.Thread | None = None

    def ensure_worker_started() -> None:
        """
        Startet den Worker genau dann, wenn er noch nicht läuft.
        """
        nonlocal worker_thread
        if worker_thread is None or not worker_thread.is_alive():
            # daemon=True:
            # Der Thread blockiert kein Prozessende, falls der Hauptprozess endet.
            worker_thread = threading.Thread(target=run_worker, daemon=True)
            worker_thread.start()

    def handle_cmd(cmd: str):
        """
        Führt ein Kommando aus, das vom Rust-Manager gesendet wurde.
        """
        if cmd == CMD_START:
            ensure_worker_started()
            return RESULT_STARTED
        if cmd == CMD_PAUSE:
            return plugin.pause()
        if cmd == CMD_RESUME:
            return plugin.resume()
        if cmd == CMD_STOP:
            return plugin.stop()
        if cmd == CMD_STATUS:
            return build_status(worker_thread, run_done)
        raise ValueError(f"{ERR_UNKNOWN_CMD_PREFIX}{cmd}")

    # Haupt-IPC-Schleife:
    # Liest Zeilen von stdin, interpretiert sie als JSON-Kommandos
    # und sendet ACKs / Fehler zurück.
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue

        request_id = None
        try:
            msg = json.loads(line)
            cmd = msg.get(CMD)
            request_id = msg.get(REQUEST_ID)

            # Sicherheitscheck:
            # Das Kommando muss für genau diese Instanz bestimmt sein.
            if msg.get(INSTANCE_ID) != instance_id:
                write_ack(instance_id, request_id, False, error=ERR_WRONG_INSTANCE_ID)
                continue

            result = handle_cmd(cmd)
            write_ack(instance_id, request_id, True, result=result)

        except Exception as e:
            write_ack(
                instance_id,
                request_id,
                False,
                error=str(e),
                trace=traceback.format_exc(),
            )

        # Wenn das Plugin fertig ist, wird genau einmal das Abschluss-Event gesendet.
        if run_done.is_set():
            try:
                emit_exited(instance_id, run_result)
            except Exception:
                pass
            return 0

    # Falls stdin geschlossen wurde, aber das Plugin schon fertig war:
    # Abschlussmeldung nachholen.
    if run_done.is_set():
        try:
            emit_exited(instance_id, run_result)
        except Exception:
            pass

    return 0


# Direkter Skriptstart: main() ausführen und Exit-Code zurückgeben.
if __name__ == MAIN__:
    raise SystemExit(main())
