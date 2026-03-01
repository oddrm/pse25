diesel::table! {
    files (path) {
        path -> Varchar,
        is_mcap -> Bool,
        is_custom_metadata -> Bool,
    }
}

diesel::table! {
    entries (id) {
        // inherent data
        id -> BigInt,
        // TODO set file name
        name -> Varchar,
        path -> Varchar,
        size -> BigInt,
        status -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        // from yaml
        time_machine -> Nullable<Double>,
        platform_name -> Nullable<Varchar>,
        platform_image_link -> Nullable<Varchar>,
        scenario_name -> Nullable<Varchar>,
        scenario_creation_time -> Nullable<Timestamptz>,
        scenario_description -> Nullable<Text>,
        sequence_duration -> Nullable<Double>,
        sequence_distance -> Nullable<Double>,
        sequence_lat_starting_point_deg -> Nullable<Double>,
        sequence_lon_starting_point_deg -> Nullable<Double>,
        weather_cloudiness -> Nullable<VarChar>,
        weather_precipitation -> Nullable<VarChar>,
        weather_precipitation_deposits -> Nullable<VarChar>,
        weather_wind_intensity -> Nullable<VarChar>,
        weather_road_humidity -> Nullable<VarChar>,
        weather_fog -> Nullable<Bool>,
        weather_snow -> Nullable<Bool>,
        tags -> Array<Text>,
    }
}

diesel::table! {
    sequences (id) {
        id -> BigInt,
        entry_id -> BigInt,
        description -> Text,
        start_timestamp -> BigInt,
        end_timestamp -> BigInt,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        tags -> Array<Text>,
    }
}

diesel::table! {
    topics (id) {
        id -> BigInt,
        entry_id -> BigInt,
        topic_name -> Varchar,
        topic_type -> Nullable<Varchar>,
        message_count -> BigInt,
        frequency -> Nullable<Double>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    sensors (id) {
        id -> BigInt,
        entry_id -> BigInt,
        sensor_name -> Varchar,
        manufacturer -> Nullable<Varchar>,
        sensor_type -> Nullable<Varchar>,
        ros_topics -> Array<Text>,
        custom_parameters -> Nullable<Jsonb>,
    }
}

diesel::joinable!(sequences -> entries (entry_id));
diesel::joinable!(sensors -> entries (entry_id));
diesel::joinable!(topics -> entries (entry_id));

diesel::allow_tables_to_appear_in_same_query!(entries, sequences, sensors, files, topics);
