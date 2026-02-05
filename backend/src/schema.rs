diesel::table! {
    files (path) {
        path -> Varchar,
        is_mcap -> Bool,
        is_custom_metadata -> Bool,
    }
}
diesel::table! {
    entries (id) {
        id -> BigInt,
        name -> Varchar,
        path -> Varchar,
        platform -> Varchar,
        size -> BigInt,
        start_time_ns -> Nullable<BigInt>,
        duration_ns -> Nullable<BigInt>,
        total_message_count -> Nullable<BigInt>,
        storage_identifier -> Nullable<Varchar>,
        compression_format -> Nullable<Varchar>,
        compression_mode -> Nullable<Varchar>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}
diesel::table! {
    topics (id) {
        id -> BigInt,
        entry_id -> BigInt,
        topic_name -> Varchar,
        message_count -> BigInt,
        type_ -> Nullable<Varchar>,
        type_description_hash -> Nullable<Varchar>,
        serialization_format -> Nullable<Varchar>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    tags (id) {
        id -> BigInt,
        entry_id -> BigInt,
        name -> Varchar,
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
    }
}

diesel::table! {
    metadata (entry_id) {
        entry_id -> BigInt,
        metadata_json -> Nullable<Jsonb>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    metadata_info (id) {
        id -> BigInt,
        metadata_id -> BigInt,
        data_spec_version -> Nullable<Varchar>,
        dataset_license -> Nullable<Varchar>,
        meta_data_spec_version -> Nullable<Varchar>,
        sequence_version -> Nullable<Varchar>,
        software_info -> Nullable<Varchar>,
        software_version -> Nullable<Varchar>,
        time_human -> Nullable<Varchar>,
        time_machine -> Nullable<Double>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    metadata_labeling (metadata_id, labeling_key) {
        metadata_id -> BigInt,
        labeling_key -> Varchar,
        creation_time_human -> Nullable<Varchar>,
        freetext -> Nullable<Text>,
        policy_version -> Nullable<Varchar>,
        provider -> Nullable<Varchar>,
        sensors_json -> Nullable<Jsonb>,
    }
}

diesel::table! {
    metadata_scenario (id) {
        id -> BigInt,
        metadata_id -> BigInt,
        environment_dynamics -> Nullable<Varchar>,
        environment_tags -> Nullable<Jsonb>,
        name -> Nullable<Varchar>,
    }
}

diesel::table! {
    metadata_dataset_sequence (id) {
        id -> BigInt,
        metadata_id -> BigInt,
        description -> Nullable<Text>,
        distance -> Nullable<Double>,
        duration -> Nullable<Double>,
        lat_starting_point_deg -> Nullable<Double>,
        lon_starting_point_deg -> Nullable<Double>,
        name -> Nullable<Varchar>,
        weather -> Nullable<Jsonb>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    metadata_setup (id) {
        id -> BigInt,
        metadata_id -> BigInt,
        name -> Nullable<Varchar>,
        platform_description_link -> Nullable<Varchar>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    metadata_sensor (id) {
        id -> BigInt,
        metadata_id -> BigInt,
        sensor_key -> Varchar,
        acquisition_rate -> Nullable<Double>,
        acquistion_mode -> Nullable<Varchar>,
        capture_rate -> Nullable<Double>,
        channel_number -> Nullable<Integer>,
        channel_space -> Nullable<Varchar>,
        firmware_version -> Nullable<Varchar>,
        focus_position_m -> Nullable<Double>,
        fov_horizontal_deg -> Nullable<Double>,
        fov_vertical_deg -> Nullable<Double>,
        frame_id -> Nullable<Varchar>,
        freetext -> Nullable<Text>,
        image_height -> Nullable<Integer>,
        image_width -> Nullable<Integer>,
        lens -> Nullable<Varchar>,
        manufacturer -> Nullable<Varchar>,
        max_exposure -> Nullable<Integer>,
        model -> Nullable<Varchar>,
        mtu -> Nullable<Integer>,
        optical_center_frame -> Nullable<Varchar>,
        ros_topics -> Nullable<Jsonb>,
        sw_trigger_rate -> Nullable<Double>,
        time_stamp_accuracy -> Nullable<Varchar>,
        time_sync_method -> Nullable<Varchar>,
        trigger_method -> Nullable<Varchar>,
        trigger_mode -> Nullable<Bool>,
        trigger_reference -> Nullable<Varchar>,
        trigger_source -> Nullable<Varchar>,
        type_ -> Nullable<Varchar>,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(tags -> entries (entry_id));
diesel::joinable!(metadata -> entries (entry_id));
diesel::joinable!(sequences -> entries (entry_id));
diesel::joinable!(topics -> entries (entry_id));
diesel::joinable!(metadata_info -> metadata (metadata_id));
diesel::joinable!(metadata_labeling -> metadata (metadata_id));
diesel::joinable!(metadata_scenario -> metadata (metadata_id));
diesel::joinable!(metadata_dataset_sequence -> metadata (metadata_id));
diesel::joinable!(metadata_setup -> metadata (metadata_id));
diesel::joinable!(metadata_sensor -> metadata (metadata_id));

diesel::allow_tables_to_appear_in_same_query!(
    entries,
    metadata,
    sequences,
    tags,
    topics,
    metadata_info,
    metadata_labeling,
    metadata_scenario,
    metadata_dataset_sequence,
    metadata_setup,
    metadata_sensor
);
