import type { Sensor } from "./sensor";
import type { Topic } from "./topic";

export interface Entry {
    id: entryID;
    name: string;
    path: string;
    size: number;
    status: string;
    created_at: string;
    updated_at: string;
    time_machine: number | null;
    platform_name: string | null;
    platform_image_link: string | null;
    scenario_name: string | null;
    scenario_creation_time: string | null;
    scenario_description: string | null;
    sequence_duration: number | null;
    sequence_distance: number | null;
    sequence_lat_starting_point_deg: number | null;
    sequence_lon_starting_point_deg: number | null;
    weather_cloudiness: string | null;
    weather_precipitation: string | null;
    weather_precipitation_deposits: string | null;
    weather_wind_intensity: string | null;
    weather_road_humidity: string | null;
    weather_fog: boolean | null;
    weather_snow: boolean | null;
    tags: string[];

    // Virtual or joined fields (not directly in backend 'entries' table but sent by some routes)
    topics?: Topic[];
    sensors?: Sensor[];
}

export type entryID = number;
