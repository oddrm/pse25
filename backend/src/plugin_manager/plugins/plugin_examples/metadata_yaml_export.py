# Metadaten für den Plugin-Manager.
PLUGIN_NAME = "metadata_yaml_export"
PLUGIN_DESCRIPTION = "Export entry metadata to YAML file."
PLUGIN_TRIGGER = "manual"

# Rückgabewert bei regulärem Ende.
STOPPED = "stopped"

from plugin_base import BasePlugin, TICK_SECONDS
import logging
import urllib.request
import json
import time

logger = logging.getLogger(__name__)


class PluginImpl(BasePlugin):
    def run(self, data: str) -> str:
        """
        Exportiert Metadaten eines oder mehrerer Einträge in YAML-Dateien.

        Erwartet optional JSON in `data`, z. B.:
        {
          "entry_path": "/data/example.mcap"
        }

        Wenn `entry_path` fehlt, werden alle Einträge exportiert.
        """
        base = "http://127.0.0.1:8080"

        def get_json(path: str):
            """
            Führt einen GET-Request gegen die Backend-API aus
            und liefert JSON als Python-Objekt zurück.
            """
            url = base + path
            req = urllib.request.Request(url, headers={"Accept": "application/json"})
            with urllib.request.urlopen(req, timeout=10) as resp:
                return json.loads(resp.read().decode())

        txid = 0
        parsed = None

        # Eingabedaten parsen.
        try:
            parsed = json.loads(data)
        except Exception:
            logger.error(
                "Plugin 'metadata_yaml_export' failed to parse input data as JSON: %s",
                data,
            )
            return STOPPED

        logger.debug("Plugin 'metadata_yaml_export' parsed input data: %s", parsed)

        entries = []

        # Falls ein konkreter Pfad angegeben ist, nur diesen einen Eintrag exportieren.
        if "entry_path" in parsed:
            try:
                entries = [get_json(f"/paths/tx/{txid}?path={parsed['entry_path']}")]
            except Exception as e:
                logger.warning(
                    "failed to fetch entry by path %s: %s", parsed.get("entry_path"), e
                )
                return STOPPED
        else:
            # Sonst alle Einträge laden.
            try:
                entries = get_json(f"/entries?txid={txid}")[0]
            except Exception as e:
                logger.warning("failed to fetch entries list: %s", e)
                return STOPPED

        logger.info("Exporting metadata for %d entries", len(entries))

        for i, entry in enumerate(entries):
            logger.debug(f"processing entry {entry}")

            if self.should_stop():
                logger.info("append_42 stopping on request")
                break

            self.wait_while_paused()

            eid = entry.get("id")
            if eid is None:
                continue

            # Vollständigen Eintrag laden, damit bei fehlenden Metadaten
            # auf das Entry-Objekt zurückgegriffen werden kann.
            try:
                entry_full = get_json(f"/entries/{eid}/tx/{txid}")
            except Exception as e:
                logger.warning("failed to fetch full entry for id %s: %s", eid, e)
                entry_full = entry

            # Metadaten laden, bei Fehler einmal neu versuchen.
            try:
                metadata = get_json(f"/entries/{eid}/metadata/tx/{txid}")
            except Exception as e:
                logger.warning(
                    "failed to fetch metadata for id %s (first try): %s", eid, e
                )
                try:
                    time.sleep(0.2)
                    metadata = get_json(f"/entries/{eid}/metadata/tx/{txid}")
                except Exception as e2:
                    logger.warning(
                        "failed to fetch metadata for id %s (retry): %s", eid, e2
                    )
                    metadata = {}

            # Sequenzen laden.
            try:
                sequences = get_json(f"/entries/{eid}/sequences/tx/{txid}")
            except Exception as e:
                logger.warning("failed to fetch sequences for id %s: %s", eid, e)
                sequences = {}

            # Sensoren laden.
            try:
                sensors = get_json(f"/entries/{eid}/sensors/tx/{txid}")
            except Exception as e:
                logger.warning("failed to fetch sensors for id %s: %s", eid, e)
                sensors = {}

            # Falls Metadaten unvollständig oder leer sind,
            # fehlende Werte aus `entry_full` ergänzen.
            if not metadata or not any(
                metadata.get(k) is not None
                for k in (
                    "time_machine",
                    "platform_name",
                    "platform_image_link",
                    "scenario_name",
                    "scenario_description",
                    "sequence_duration",
                )
            ):
                for k in [
                    "time_machine",
                    "platform_name",
                    "platform_image_link",
                    "scenario_name",
                    "scenario_description",
                    "sequence_duration",
                    "sequence_distance",
                    "sequence_lat_starting_point_deg",
                    "sequence_lon_starting_point_deg",
                    "weather_cloudiness",
                    "weather_precipitation",
                    "weather_precipitation_deposits",
                    "weather_wind_intensity",
                    "weather_road_humidity",
                    "weather_fog",
                    "weather_snow",
                    "topics",
                ]:
                    if metadata.get(k) is None and entry_full.get(k) is not None:
                        metadata[k] = entry_full.get(k)

            # YAML-Struktur im erwarteten Zielschema aufbauen.
            defs = {}

            # -------- info --------
            info = {}
            if metadata.get("time_machine") is not None:
                info["time_machine"] = metadata.get("time_machine")
            if info:
                defs["info"] = info

            # -------- setup --------
            setup = {}
            if metadata.get("platform_name"):
                setup["name"] = metadata.get("platform_name")
            if metadata.get("platform_image_link"):
                setup["platform_image_link"] = metadata.get("platform_image_link")
            if setup:
                defs["setup"] = setup

            # -------- scenario --------
            scenario = {}
            if metadata.get("scenario_name"):
                scenario["name"] = metadata.get("scenario_name")
            if metadata.get("scenario_description"):
                scenario["description"] = metadata.get("scenario_description")
            if scenario:
                defs["scenario"] = scenario

            # -------- sequence --------
            sequence_node = {}
            if metadata.get("sequence_duration") is not None:
                sequence_node["duration"] = metadata.get("sequence_duration")
            if metadata.get("sequence_distance") is not None:
                sequence_node["distance"] = metadata.get("sequence_distance")
            if metadata.get("sequence_lat_starting_point_deg") is not None:
                sequence_node["lat_starting_point_deg"] = metadata.get(
                    "sequence_lat_starting_point_deg"
                )
            if metadata.get("sequence_lon_starting_point_deg") is not None:
                sequence_node["lon_starting_point_deg"] = metadata.get(
                    "sequence_lon_starting_point_deg"
                )

            # Wetter als verschachteltes Objekt aufbauen.
            weather_obj = {}
            if metadata.get("weather_cloudiness") is not None:
                weather_obj["cloudiness"] = metadata.get("weather_cloudiness")
            if metadata.get("weather_precipitation") is not None:
                weather_obj["precipitation"] = metadata.get("weather_precipitation")
            if metadata.get("weather_precipitation_deposits") is not None:
                weather_obj["precipitation_deposits"] = metadata.get(
                    "weather_precipitation_deposits"
                )
            if metadata.get("weather_wind_intensity") is not None:
                weather_obj["wind_intensity"] = metadata.get("weather_wind_intensity")
            if metadata.get("weather_road_humidity") is not None:
                weather_obj["road_humidity"] = metadata.get("weather_road_humidity")
            if metadata.get("weather_fog") is not None:
                weather_obj["fog"] = metadata.get("weather_fog")
            if metadata.get("weather_snow") is not None:
                weather_obj["snow"] = metadata.get("weather_snow")

            if weather_obj:
                sequence_node["weather"] = weather_obj

            # Tags aus dem Entry übernehmen.
            tags = entry.get("tags")
            if tags:
                sequence_node["tags"] = tags

            if sequence_node:
                defs["sequence"] = sequence_node

            # -------- subsequence --------
            # Untersequenzen aus der Sequence-Map aufbauen.
            subseqs = []
            try:
                for _id, seq in (
                    sequences.items() if isinstance(sequences, dict) else []
                ):
                    s = {}
                    if seq.get("start_timestamp") is not None:
                        s["start_time_machine"] = seq.get("start_timestamp")
                    if seq.get("end_timestamp") is not None:
                        s["end_time"] = seq.get("end_timestamp")
                    if seq.get("description"):
                        s["description"] = seq.get("description")
                    if seq.get("tags"):
                        s["tags"] = seq.get("tags")
                    if s:
                        subseqs.append(s)
            except Exception:
                subseqs = []

            if subseqs:
                defs["subsequence"] = subseqs

            # -------- sensors --------
            # Sensoren immer als Mapping ausgeben, auch wenn leer.
            sensors_out = {}
            try:
                for _id, s in sensors.items() if isinstance(sensors, dict) else []:
                    name = s.get("sensor_name") or s.get("sensor") or str(_id)
                    sensor_obj = {}

                    if s.get("manufacturer"):
                        sensor_obj["manufacturer"] = s.get("manufacturer")
                    if s.get("sensor_type"):
                        sensor_obj["type"] = s.get("sensor_type")
                    if s.get("ros_topics"):
                        sensor_obj["ros_topics"] = s.get("ros_topics")

                    # Benutzerdefinierte Parameter ergänzen,
                    # aber bekannte Schlüssel nicht überschreiben.
                    cp = s.get("custom_parameters")
                    if isinstance(cp, dict):
                        for k2, v2 in cp.items():
                            if k2 in ("manufacturer", "type", "ros_topics"):
                                continue
                            sensor_obj[k2] = v2

                    sensors_out[name] = sensor_obj
            except Exception:
                sensors_out = {}

            defs["sensors"] = sensors_out

            # Gesamte YAML-Struktur.
            yaml_struct = {
                "title": f"{entry.get('name')}.exported.yaml",
                "description": f"Exported metadata for {entry.get('name')}",
                "definitions": defs,
            }

            # Für robustes YAML bevorzugt PyYAML verwenden.
            # Falls nicht vorhanden, wird versucht, es zur Laufzeit zu installieren.
            try:
                import yaml
            except Exception:
                try:
                    import sys, subprocess

                    subprocess.check_call(
                        [sys.executable, "-m", "pip", "install", "pyyaml"]
                    )
                    import importlib

                    yaml = importlib.import_module("yaml")
                except Exception:
                    logger.exception(
                        "failed to import or install PyYAML; falling back to simple dump"
                    )
                    yaml = None

            if yaml:
                try:
                    yaml_text = yaml.safe_dump(
                        yaml_struct, sort_keys=False, allow_unicode=True
                    )
                except Exception:
                    logger.exception("PyYAML dump failed; falling back to str()")
                    yaml_text = str(yaml_struct)
            else:
                # Letzter Fallback: Python-Stringrepräsentation schreiben.
                yaml_text = str(yaml_struct)

            # YAML-Datei im selben Verzeichnis wie die MCAP-Datei ablegen.
            try:
                import os

                mcap_path = entry.get("path")
                if mcap_path:
                    dirp = os.path.dirname(mcap_path)
                    mcap_base = os.path.basename(mcap_path)
                    out_name = mcap_base + ".exported.yaml"
                    out_path = os.path.join(dirp, out_name)

                    with open(out_path, "w", encoding="utf-8") as fh:
                        fh.write(yaml_text)

                    logger.info("Wrote metadata YAML for entry %s -> %s", eid, out_path)
                    logger.debug("YAML content for entry %s:\n%s", eid, yaml_text)
                else:
                    logger.warning("Entry %s has no path; skipping file write", eid)
            except Exception:
                logger.exception("failed to write YAML for entry %s", eid)

            # Fortschritt an den Runner melden.
            logger.info(f"PROGRESS:{(i + 1)/len(entries):.2f}")

            time.sleep(TICK_SECONDS)

        return STOPPED
