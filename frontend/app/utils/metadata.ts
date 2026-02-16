export interface MetadataWeb {
    time_machine?: number;
    platform_name?: string;
    platform_image_link?: string;
    scenario_name?: string;
    scenario_creation_time?: string;
    scenario_description?: string;
    sequence_duration?: number;
    sequence_distance?: number;
    sequence_lat_starting_point_deg?: number;
    sequence_lon_starting_point_deg?: number;
    weather_cloudiness?: string;
    weather_precipitation?: string;
    weather_precipitation_deposits?: string;
    weather_wind_intensity?: string;
    weather_road_humidity?: string;
    weather_fog?: boolean;
    weather_snow?: boolean;
    topics?: string[];
}
