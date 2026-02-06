
CREATE TABLE metadata_info (
    id BIGSERIAL PRIMARY KEY,
    metadata_id BIGINT NOT NULL UNIQUE REFERENCES metadata(id) ON DELETE CASCADE,
    data_spec_version VARCHAR,
    dataset_license VARCHAR,
    meta_data_spec_version VARCHAR,
    sequence_version VARCHAR,
    software_info VARCHAR,
    software_version VARCHAR,
    time_human VARCHAR,
    time_machine DOUBLE PRECISION,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_metadata_info_metadata_id ON metadata_info(metadata_id);


CREATE TABLE metadata_labeling (
    labeling_key VARCHAR PRIMARY KEY,
    metadata_id BIGINT NOT NULL REFERENCES metadata(id) ON DELETE CASCADE,
    creation_time_human VARCHAR,
    freetext TEXT,
    policy_version VARCHAR,
    provider VARCHAR,
    sensors_json JSONB,
    UNIQUE (metadata_id, labeling_key)
);

CREATE INDEX idx_metadata_labeling_metadata_id ON metadata_labeling(metadata_id);



CREATE TABLE metadata_scenario (
    id BIGSERIAL PRIMARY KEY,
    metadata_id BIGINT NOT NULL UNIQUE REFERENCES metadata(id) ON DELETE CASCADE,
    environment_dynamics VARCHAR,
    environment_tags JSONB, --ist das JSON????
    name VARCHAR
);

CREATE INDEX idx_metadata_scenario_metadata_id ON metadata_scenario(metadata_id);
CREATE INDEX idx_metadata_scenario_name ON metadata_scenario(name);


CREATE TABLE metadata_dataset_sequence (
    id BIGSERIAL PRIMARY KEY,
    metadata_id BIGINT NOT NULL UNIQUE REFERENCES metadata(id) ON DELETE CASCADE,
    description TEXT,
    distance DOUBLE PRECISION,
    duration DOUBLE PRECISION,
    lat_starting_point_deg DOUBLE PRECISION,
    lon_starting_point_deg DOUBLE PRECISION,
    name VARCHAR,
    weather JSONB,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_metadata_dataset_sequence_metadata_id ON metadata_dataset_sequence(metadata_id);
CREATE INDEX idx_metadata_dataset_sequence_name ON metadata_dataset_sequence(name);


CREATE TABLE metadata_setup (
    id BIGSERIAL PRIMARY KEY,
    metadata_id BIGINT NOT NULL UNIQUE REFERENCES metadata(id) ON DELETE CASCADE,
    name VARCHAR,
    platform_description_link VARCHAR,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_metadata_setup_metadata_id ON metadata_setup(metadata_id);


CREATE TABLE metadata_sensor (
    metadata_id BIGINT NOT NULL REFERENCES metadata(id) ON DELETE CASCADE,
    sensor_key VARCHAR PRIMARY KEY,
    acquisition_rate DOUBLE PRECISION,
    acquistion_mode VARCHAR,
    capture_rate DOUBLE PRECISION,
    channel_number INT,
    channel_space VARCHAR,
    firmware_version VARCHAR,
    focus_position_m DOUBLE PRECISION,
    fov_horizontal_deg DOUBLE PRECISION,
    fov_vertical_deg DOUBLE PRECISION,
    frame_id VARCHAR,
    freetext TEXT,
    image_height INT,
    image_width INT,
    lens VARCHAR,
    manufacturer VARCHAR,
    max_exposure INT,
    model VARCHAR,
    mtu INT,
    optical_center_frame VARCHAR,
    ros_topics JSONB, --is it true????
    sw_trigger_rate DOUBLE PRECISION,
    time_stamp_accuracy VARCHAR,
    time_sync_method VARCHAR,
    trigger_method VARCHAR,
    trigger_mode BOOLEAN,
    trigger_reference VARCHAR,
    trigger_source VARCHAR,
    type VARCHAR,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (metadata_id, sensor_key)
);

CREATE INDEX idx_metadata_sensor_metadata_id ON metadata_sensor(metadata_id);
CREATE INDEX idx_metadata_sensor_type ON metadata_sensor(type);
CREATE INDEX idx_metadata_sensor_frame_id ON metadata_sensor(frame_id);



