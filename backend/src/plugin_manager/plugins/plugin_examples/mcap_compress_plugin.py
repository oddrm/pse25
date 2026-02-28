PLUGIN_NAME = "mcap_compress"
PLUGIN_DESCRIPTION = "Create a compressed MCAP file from an existing .mcap using the `mcap` CLI (chunk compression)."
PLUGIN_TRIGGER = "manual"
STOPPED = "stopped"

from plugin_base import BasePlugin
import json
import logging
import subprocess
import time
from pathlib import Path
from typing import Any

logger = logging.getLogger(__name__)


def _extract_payload(data: str) -> Any:
    s = (data or "").strip()
    if not s:
        return None
    try:
        return json.loads(s)
    except json.JSONDecodeError:
        # fallback, should rarely happen in your backend because it forces object payloads
        return s


def _extract_options(data: str) -> tuple[Path | None, str, float | None, bool]:
    obj = _extract_payload(data)

    out_path: Path | None = None
    compression = "zstd"
    stall_timeout_s: float | None = None
    already_compressed = False  # NEW

    if isinstance(obj, dict):
        o = obj.get("out_path")
        if isinstance(o, str) and o.strip():
            out_path = Path(o.strip())

        c = obj.get("compression")
        if isinstance(c, str) and c.strip():
            compression = c.strip()

        st = obj.get("stall_timeout_s")
        if isinstance(st, (int, float)) and float(st) > 0:
            stall_timeout_s = float(st)

        ac = obj.get("already_compressed")
        if isinstance(ac, bool):
            already_compressed = ac

    return out_path, compression, stall_timeout_s, already_compressed


def _safe_size_bytes(p: Path) -> int:
    try:
        return p.stat().st_size
    except Exception:
        return 0


def _with_compressed_suffix(p: Path) -> Path:
    # foo.mcap -> foo.compressed.mcap
    if p.suffix.lower() == ".mcap":
        return p.with_name(p.stem + ".compressed" + p.suffix)
    return Path(str(p) + ".compressed")


def _default_output_path_next_to_mcap(mcap_path: Path) -> Path:
    return mcap_path.with_name(mcap_path.stem + ".compressed" + mcap_path.suffix)


def _extract_mcap_path(data: str) -> Path:
    obj = _extract_payload(data)
    if isinstance(obj, dict):
        p = obj.get("mcap_path")
        if isinstance(p, str) and p.strip():
            return Path(p.strip())
        raise ValueError("missing 'mcap_path' in JSON payload")
    if isinstance(obj, str) and obj.strip():
        return Path(obj.strip())
    raise ValueError("empty data: expected JSON with 'mcap_path'")


class PluginImpl(BasePlugin):
    """
    Wie mcap_info_gzip_plugin:
    - erwartet eine einzelne MCAP-Datei
    - schreibt Output standardmäßig direkt neben die Eingabedatei
    Payload:
      - JSON object (Route erzwingt Object):
          {
            "mcap_path": "/data/.../file.mcap",
            "compression": "zstd",
            "out_path": "/data/.../file.compressed.mcap",   // optional
            "stall_timeout_s": 120                          // optional
          }
    """

    def run(self, data: str) -> str:
        try:
            mcap_path = _extract_mcap_path(data)
            out_path, compression, stall_timeout_s, already_compressed = _extract_options(data)
        except Exception:
            logger.exception("failed to parse input data")
            return STOPPED

        if not mcap_path.exists():
            logger.error("mcap not found: %s", mcap_path)
            return STOPPED

        # NEW: If filename already indicates compression -> treat as success, just log.
        if mcap_path.suffix.lower() == ".mcap" and mcap_path.stem.lower().endswith(".compressed"):
            logger.info("input already looks compressed, skipping: %s", mcap_path)
            return STOPPED

        # NEW: allow "already compressed, just rename" mode (useful if UI points to a compressed file without suffix)
        if already_compressed:
            new_path = _with_compressed_suffix(mcap_path)
            if new_path.resolve() == mcap_path.resolve():
                logger.info("already_compressed=true but name already ok, nothing to do: %s", mcap_path)
                return STOPPED

            try:
                if new_path.exists():
                    logger.error("cannot rename: target already exists: %s", new_path)
                    return STOPPED
                mcap_path.replace(new_path)
            except Exception:
                logger.exception("failed to rename already-compressed file: %s -> %s", mcap_path, new_path)
                return STOPPED

            logger.info("file was already compressed; renamed to: %s", new_path)
            return STOPPED

        # NEW: guard against double-compressing already-compressed outputs
        # foo.compressed.mcap -> reject
        if mcap_path.suffix.lower() == ".mcap" and mcap_path.stem.lower().endswith(".compressed"):
            logger.error(
                "refusing to compress an already-compressed file: %s (pass the original *.mcap instead)",
                mcap_path,
            )
            return STOPPED

        if out_path is None:
            out_path = _default_output_path_next_to_mcap(mcap_path)

        # NEW: also guard against overwriting the input
        if out_path.resolve() == mcap_path.resolve():
            logger.error("output path equals input path (refusing to overwrite): %s", out_path)
            return STOPPED

        # NEW: write to temp file first to avoid watcher parsing half-written .mcap
        tmp_out_path = Path(str(out_path) + ".tmp")

        stdout_txt = ""
        stderr_txt = ""

        self.wait_while_paused()
        if self.should_stop():
            logger.info("%s stopping on request (before start)", PLUGIN_NAME)
            return STOPPED

        try:
            out_path.parent.mkdir(parents=True, exist_ok=True)
        except Exception:
            logger.exception("failed to create output directory: %s", out_path.parent)
            return STOPPED

        # NEW: remove stale temp + final (optional: keep final if you prefer)
        try:
            if tmp_out_path.exists():
                tmp_out_path.unlink()
        except Exception:
            logger.exception("failed to remove existing temp output file: %s", tmp_out_path)
            return STOPPED

        # IMPORTANT: mcap v0.0.61 uses --output/-o
        cmd = [
            "mcap",
            "compress",
            str(mcap_path),
            "--output",
            str(tmp_out_path),
            "--compression",
            compression,
        ]
        logger.info("running: %s", " ".join(cmd))

        start_ts = time.monotonic()
        last_log_ts = start_ts
        last_progress_ts = start_ts
        last_size = _safe_size_bytes(tmp_out_path)

        progress_interval_s = 5.0

        proc = None  # NEW: prevent NameError

        try:
            proc = subprocess.Popen(
                cmd,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

            # Progress loop only runs if proc started successfully
            while True:
                self.wait_while_paused()
                if self.should_stop():
                    logger.warning("%s stopping on request; terminating mcap process", PLUGIN_NAME)
                    try:
                        proc.terminate()
                    except Exception:
                        pass
                    break

                rc = proc.poll()
                now = time.monotonic()

                if stall_timeout_s is not None and (now - last_progress_ts) >= stall_timeout_s:
                    logger.error(
                        "stall timeout: no output growth for %.1fs (tmp_exists=%s tmp_size=%d). terminating mcap process",
                        now - last_progress_ts,
                        tmp_out_path.exists(),
                        _safe_size_bytes(tmp_out_path),
                    )
                    try:
                        proc.terminate()
                    except Exception:
                        pass
                    break

                if now - last_log_ts >= progress_interval_s:
                    exists = tmp_out_path.exists()
                    cur_size = _safe_size_bytes(tmp_out_path)
                    elapsed = now - start_ts

                    if cur_size != last_size:
                        logger.info(
                            "progress: elapsed=%.1fs tmp_exists=%s tmp_size=%d bytes (+%d)",
                            elapsed,
                            exists,
                            cur_size,
                            cur_size - last_size,
                        )
                        last_size = cur_size
                        last_progress_ts = now
                    else:
                        logger.info(
                            "progress: elapsed=%.1fs tmp_exists=%s tmp_size=%d bytes (no change)",
                            elapsed,
                            exists,
                            cur_size,
                        )
                    last_log_ts = now

                if rc is not None:
                    break

                time.sleep(0.2)

        except FileNotFoundError:
            logger.error("'mcap' CLI not found in PATH")
            return STOPPED
        except Exception:
            logger.exception("failed to spawn/run `mcap compress`")
            return STOPPED
        finally:
            if proc is not None:
                try:
                    stdout_b, stderr_b = proc.communicate()
                    stdout_txt = (stdout_b or b"").decode("utf-8", errors="replace").strip()
                    stderr_txt = (stderr_b or b"").decode("utf-8", errors="replace").strip()
                except Exception:
                    pass

        returncode = proc.returncode if proc is not None and proc.returncode is not None else -1

        if returncode != 0:
            logger.error(
                "mcap compress failed (code=%s). stderr=%s stdout=%s",
                returncode,
                stderr_txt,
                stdout_txt,
            )
            return STOPPED

        if not tmp_out_path.exists():
            logger.error(
                "mcap compress reported success (code=0) but temp output file is missing: %s (stderr=%s stdout=%s)",
                tmp_out_path,
                stderr_txt,
                stdout_txt,
            )
            return STOPPED

        tmp_size = _safe_size_bytes(tmp_out_path)
        if tmp_size <= 0:
            try:
                tmp_out_path.unlink()
            except Exception:
                pass
            logger.error(
                "mcap compress reported success (code=0) but temp output size is %d bytes: %s (stderr=%s stdout=%s)",
                tmp_size,
                tmp_out_path,
                stderr_txt,
                stdout_txt,
            )
            return STOPPED

        # NEW: atomic finalize (replace existing final file)
        try:
            if out_path.exists():
                out_path.unlink()
            tmp_out_path.replace(out_path)
        except Exception:
            logger.exception("failed to move temp output into final location: %s -> %s", tmp_out_path, out_path)
            return STOPPED

        elapsed = time.monotonic() - start_ts
        out_size = _safe_size_bytes(out_path)
        logger.info("wrote compressed MCAP: %s (size=%d bytes, elapsed=%.1fs)", out_path, out_size, elapsed)
        return STOPPED