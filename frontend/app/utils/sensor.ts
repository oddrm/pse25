export interface Sensor {
    id: number;
    entry_id: number;
    sensor_name: string;
    manufacturer: string | null;
    sensor_type: string | null;
    ros_topics: string[];
    custom_parameters: any | null;
}

export interface SensorWeb {
    sensor_name: string;
    manufacturer: string | null;
    sensor_type: string | null;
    ros_topics: string[];
    custom_parameters: any | null;
}
