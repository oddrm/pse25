from __future__ import annotations

import argparse
import importlib.util
import json
import sys
import threading
import traceback
from pathlib import Path
from typing import Any, Final, Literal, NotRequired, TypedDict

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

# Rückgabe Status
def build_status(worker_thread: threading.Thread | None, run_done: threading.Event) -> dict:
    return {
        STATUS_RUNNING: worker_thread is not None and worker_thread.is_alive(),
        STATUS_STOPPED: run_done.is_set(),
    }


# Worker ist fertig -> Evaluation
def emit_exited(instance_id: int, run_result: "RunResult") -> None:
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
    plugin_file = Path(plugin_path).resolve()
    plugin_dir = str(plugin_file.parent)

    if plugin_dir not in sys.path:
        sys.path.insert(0, plugin_dir)

    # Eindeutiger Modulname pro Plugin-Datei, um Kollisionen in sys.modules zu vermeiden.
    # (Gleichnamige Plugins/mehrfaches Laden im selben Prozess sind damit unkritischer.)
    unique_name = f"user_plugin_{plugin_file.stem}_{abs(hash(str(plugin_file)))}"

    # Läd exakt dieses File -> Bauplan
    spec = importlib.util.spec_from_file_location(unique_name, str(plugin_file))
    # kein Loader oder Lademechandismus auffindbar -> richtig laden
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Cannot load plugin from {plugin_path}")
    module = importlib.util.module_from_spec(spec)
    # Initialisiert das Modul, führt den Code aus -> Instanziierung
    spec.loader.exec_module(module)
    return module


def write_msg(obj: dict) -> None:
    # Zeilenweises lesen mit Handeln von Umlauten ect.
    sys.stdout.write(json.dumps(obj, ensure_ascii=False) + "\n")
    # Nachricht direkt raus -> wichtig für Inter-Process-Communication
    sys.stdout.flush()


def write_ack(
    instance_id: int,
    request_id: str | None,
    ok: bool,
    result=None,
    error: str | None = None,
    trace: str | None = None,
) -> None:
    msg = {
        INSTANCE_ID: instance_id,
        REQUEST_ID: request_id,
        OK: ok,
    }
    if ok:
        # wenn erfolgreich -> Ergebnis zurückgeben, Fehler/ Trace weglassen
        msg[RESULT] = result
    else:
        msg[ERROR] = error or UNKNOWN_ERROR
        if trace:
            msg[TRACE] = trace
    write_msg(msg)

# welche Keys in run_result -> Strukturierung
class RunResult(TypedDict):
    instance_id: int
    request_id: str
    ok: bool
    result: NotRequired[Any]
    error: NotRequired[str]
    trace: NotRequired[str]
    event: NotRequired[str]


def main() -> int:
    ap = argparse.ArgumentParser()
    # Parameter von Manager (Rust)
    ap.add_argument(PATH, required=True)
    ap.add_argument(ID, required=True, type=int)
    ap.add_argument(DATA, default="")
    args = ap.parse_args()

    # welche Instanz
    instance_id = args.instance_id

    try:
        # läd Modul aus Datei
        module = load_plugin(args.plugin_path)
        # bekommt Faktory/ Klase
        plugin_impl = getattr(module, PLUGIN_IMPL)
        # Instanz erzeugen
        plugin = plugin_impl(args.plugin_path)
    except Exception as e:
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

    # Worker fertig
    run_done = threading.Event()
    # seq/ request_id
    seq = 0
    request_id = f"{instance_id}-{seq}"
    seq += 1

    run_result: RunResult = {
        INSTANCE_ID: instance_id,
        REQUEST_ID: request_id,
        OK: True,
    }

    def run_worker():
        try:
            # run!
            res = plugin.run(args.data)
            run_result[RESULT] = res
        except Exception as exception:
            run_result[OK] = False
            run_result[ERROR] = str(exception)
            run_result[TRACE] = traceback.format_exc()
        finally:
            # stoppen
            run_done.set()
            # an Rust zurück
            emit_exited(instance_id, run_result)

    # Historie laden
    worker_thread: threading.Thread | None = None

    def ensure_worker_started() -> None:
        nonlocal worker_thread
        if worker_thread is None or not worker_thread.is_alive():
            # deamon = true -> Thread verhindert kein Prozessende
            worker_thread = threading.Thread(target=run_worker, daemon=True)
            worker_thread.start()

    def handle_cmd(cmd: str):
        if cmd == CMD_START:
            ensure_worker_started()
            return RESULT_STARTED
        if cmd == CMD_PAUSE:
            # jeweils Delegation an Plugin-Methoden
            return plugin.pause()
        if cmd == CMD_RESUME:
            return plugin.resume()
        if cmd == CMD_STOP:
            return plugin.stop()
        if cmd == CMD_STATUS:
            return build_status(worker_thread, run_done)
        raise ValueError(f"{ERR_UNKNOWN_CMD_PREFIX}{cmd}")

    for line in sys.stdin:
        # leere Zeilen ignorieren
        line = line.strip()
        if not line:
            continue

        request_id = None
        try:
            # Suche
            msg = json.loads(line)
            cmd = msg.get(CMD)
            request_id = msg.get(REQUEST_ID)

            # Test richtige Instanz
            if msg.get(INSTANCE_ID) != instance_id:
                write_ack(instance_id, request_id, False, error=ERR_WRONG_INSTANCE_ID)
                continue

            # command ausführen
            result = handle_cmd(cmd)
            write_ack(instance_id, request_id, True, result=result)

        # Fehlerbehandlung von oben
        except Exception as e:
            write_ack(
                instance_id,
                request_id,
                False,
                error=str(e),
                trace=traceback.format_exc(),
            )

        # sauberes Ende
        if run_done.is_set():
            return 0

    return 0

# wenn Skript direkt ausgeführt wird -> start main()
if __name__ == MAIN__:
    raise SystemExit(main())