PLUGIN_NAME = "append_42"
PLUGIN_DESCRIPTION = "Append 42 to description of all entries"
PLUGIN_TRIGGER = "manual"
STOPPED = "stopped"
from plugin_base import BasePlugin, TICK_SECONDS
import logging
import urllib.request
import json
import time

logger = logging.getLogger(__name__)


class PluginImpl(BasePlugin):
    def run(self, data: str) -> str:
        base = "http://127.0.0.1:8080"

        def get_json(path: str):
            url = base + path
            req = urllib.request.Request(url, headers={"Accept": "application/json"})
            with urllib.request.urlopen(req, timeout=10) as resp:
                return json.loads(resp.read().decode())

        def put_json(path: str, obj):
            url = base + path
            body = json.dumps(obj, default=str).encode("utf-8")
            req = urllib.request.Request(
                url, data=body, headers={"Content-Type": "application/json"}
            )
            req.get_method = lambda: "PUT"
            with urllib.request.urlopen(req, timeout=10) as resp:
                return resp.getcode()

        txid = 0
        entries = get_json(f"/entries?txid={txid}")[0]
        logger.info("append_42 got %d entries", len(entries))
        for e in entries:
            if self.should_stop():
                logger.info("append_42 stopping on request")
                break
            self.wait_while_paused()
            eid = e.get("id")
            if eid is None:
                continue

            entry_full = get_json(f"/entries/{eid}/tx/{txid}")

            md = {
                "time_machine": entry_full.get("time_machine"),
                "platform_name": entry_full.get("platform_name"),
                "platform_image_link": entry_full.get("platform_image_link"),
                "scenario_name": entry_full.get("scenario_name"),
                "scenario_creation_time": entry_full.get("scenario_creation_time"),
                "scenario_description": None,
                "sequence_duration": entry_full.get("sequence_duration"),
                "sequence_distance": entry_full.get("sequence_distance"),
                "sequence_lat_starting_point_deg": entry_full.get(
                    "sequence_lat_starting_point_deg"
                ),
                "sequence_lon_starting_point_deg": entry_full.get(
                    "sequence_lon_starting_point_deg"
                ),
                "weather_cloudiness": entry_full.get("weather_cloudiness"),
                "weather_precipitation": entry_full.get("weather_precipitation"),
                "weather_precipitation_deposits": entry_full.get(
                    "weather_precipitation_deposits"
                ),
                "weather_wind_intensity": entry_full.get("weather_wind_intensity"),
                "weather_road_humidity": entry_full.get("weather_road_humidity"),
                "weather_fog": entry_full.get("weather_fog"),
                "weather_snow": entry_full.get("weather_snow"),
                "topics": None,
            }

            desc = entry_full.get("scenario_description")
            if desc is None:
                newdesc = "42"
            else:
                newdesc = f"{desc}42"
            md["scenario_description"] = newdesc

            try:
                put_json(f"/entries/{eid}/metadata/tx/{txid}", md)
                logger.info("updated entry %s description", eid)
            except Exception:
                logger.exception("failed to update entry %s", eid)

            time.sleep(TICK_SECONDS)

        return STOPPED
