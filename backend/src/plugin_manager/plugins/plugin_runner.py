# python/plugin_runner.py
from __future__ import annotations

import argparse
import importlib.util
import json
import sys
import threading
import traceback
from typing import TypedDict, NotRequired, Any
from pathlib import Path


def load_plugin(plugin_path: str):
    plugin_file = Path(plugin_path).resolve()
    plugin_dir = str(plugin_file.parent)

    # Wichtig: damit "from plugin_base import BasePlugin" funktioniert
    if plugin_dir not in sys.path:
        sys.path.insert(0, plugin_dir)

    spec = importlib.util.spec_from_file_location("user_plugin", str(plugin_file))
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Cannot load plugin from {plugin_path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def write_msg(obj: dict) -> None:
    sys.stdout.write(json.dumps(obj, ensure_ascii=False) + "\n")
    sys.stdout.flush()


def write_ack(instance_id: int, request_id: str | None, ok: bool, result=None, error: str | None = None, trace: str | None = None) -> None:
    msg = {
        "instance_id": instance_id,
        "request_id": request_id,
        "ok": ok,
    }
    if ok:
        msg["result"] = result
    else:
        msg["error"] = error or "unknown_error"
        if trace:
            msg["trace"] = trace
    write_msg(msg)


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
    ap.add_argument("--plugin-path", required=True)
    ap.add_argument("--instance-id", required=True, type=int)
    ap.add_argument("--data", default="")
    args = ap.parse_args()

    instance_id = args.instance_id

    try:
        module = load_plugin(args.plugin_path)
        plugin_impl = getattr(module, "pluginImpl")
        plugin = plugin_impl(args.plugin_path)
    except Exception as e:
        write_msg({
            "instance_id": instance_id,
            "event": "init_error",
            "ok": False,
            "error": str(e),
            "trace": traceback.format_exc(),
        })
        return 1

    run_done = threading.Event()
    # python
    # Zähler für lokale Request-IDs initialisieren
    seq = 0

    # erste Run-Request-ID erzeugen und Zähler inkrementieren
    request_id = f"{instance_id}-{seq}"
    seq += 1

    # ab hier darfst du RunResult verwenden
    run_result: RunResult = {
        "instance_id": instance_id,
        "request_id": request_id,
        "ok": True,
    }# ab hier darfst du RunResult verwenden
    run_result: RunResult = {
        "instance_id": instance_id,
        "request_id": request_id,
        "ok": True,
    }

    def run_worker():
        try:
            res = plugin.run(args.data)
            run_result["result"] = res
        except Exception as exception:
            run_result["ok"] = False
            run_result["error"] = str(exception)
            run_result["trace"] = traceback.format_exc()
        finally:
            run_done.set()
            write_msg({
                "instance_id": instance_id,
                "event": "exited",
                "ok": run_result["ok"],
                "result": run_result.get("result"),
                "error": run_result.get("error"),
                "trace": run_result.get("trace"),
            })

    worker_thread: threading.Thread | None = None

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue

        request_id = None
        try:
            msg = json.loads(line)
            cmd = msg.get("cmd")
            request_id = msg.get("request_id")

            if msg.get("instance_id") != instance_id:
                write_ack(instance_id, request_id, False, error="wrong_instance_id")
                continue

            if cmd == "start":
                if worker_thread is None or not worker_thread.is_alive():
                    worker_thread = threading.Thread(target=run_worker, daemon=True)
                    worker_thread.start()
                write_ack(instance_id, request_id, True, result="started")

            elif cmd == "pause":
                write_ack(instance_id, request_id, True, result=plugin.pause())

            elif cmd == "resume":
                write_ack(instance_id, request_id, True, result=plugin.resume())

            elif cmd == "stop":
                # Soft stop: setzt nur Event; run() endet kooperativ
                write_ack(instance_id, request_id, True, result=plugin.stop())

            elif cmd == "status":
                write_ack(instance_id, request_id, True, result={
                    "running": worker_thread is not None and worker_thread.is_alive(),
                    "stopped": run_done.is_set(),
                })

            else:
                write_ack(instance_id, request_id, False, error=f"unknown_cmd:{cmd}")

        except Exception as e:
            # WICHTIG: immer mit request_id antworten (sonst wartet Rust bis Timeout)
            write_ack(
                instance_id,
                request_id,
                False,
                error=str(e),
                trace=traceback.format_exc(),
            )

        if run_done.is_set():
            return 0

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
