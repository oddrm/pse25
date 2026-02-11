-- This file should undo anything in `up.sql`
ALTER TABLE "entries" DROP COLUMN "sequence_lon_starting_point_deg";
ALTER TABLE "entries" DROP COLUMN "scenario_name";
ALTER TABLE "entries" DROP COLUMN "scenario_description";
ALTER TABLE "entries" DROP COLUMN "sequence_lat_starting_point_deg";
ALTER TABLE "entries" DROP COLUMN "tags";
ALTER TABLE "entries" DROP COLUMN "sequence_duration";
ALTER TABLE "entries" DROP COLUMN "weather_road_humidity";
ALTER TABLE "entries" DROP COLUMN "time_machine";
ALTER TABLE "entries" DROP COLUMN "platform_name";
ALTER TABLE "entries" DROP COLUMN "scenario_creation_time";
ALTER TABLE "entries" DROP COLUMN "weather_cloudiness";
ALTER TABLE "entries" DROP COLUMN "weather_precipitation";
ALTER TABLE "entries" DROP COLUMN "weather_wind_intensity";
ALTER TABLE "entries" DROP COLUMN "weather_fog";
ALTER TABLE "entries" DROP COLUMN "weather_precipitation_deposits";
ALTER TABLE "entries" DROP COLUMN "platform_image_link";
ALTER TABLE "entries" DROP COLUMN "weather_snow";
ALTER TABLE "entries" DROP COLUMN "sequence_distance";
ALTER TABLE "entries" DROP COLUMN "topics";
ALTER TABLE "entries" ADD COLUMN "platform" VARCHAR NOT NULL;
ALTER TABLE "entries" ADD COLUMN "start_time_ns" INT8;
ALTER TABLE "entries" ADD COLUMN "duration_ns" INT8;
ALTER TABLE "entries" ADD COLUMN "total_message_count" INT8;
ALTER TABLE "entries" ADD COLUMN "storage_identifier" VARCHAR;
ALTER TABLE "entries" ADD COLUMN "compression_format" VARCHAR;
ALTER TABLE "entries" ADD COLUMN "compression_mode" VARCHAR;


CREATE TABLE "metadata"(
	"id" INT8 NOT NULL PRIMARY KEY,
	"entry_id" INT8 NOT NULL,
	"metadata_json" JSONB,
	"created_at" TIMESTAMPTZ NOT NULL,
	"updated_at" TIMESTAMPTZ NOT NULL,
	FOREIGN KEY ("entry_id") REFERENCES "entries"("id")
);

CREATE TABLE "metadata_dataset_sequence"(
	"id" INT8 NOT NULL PRIMARY KEY,
	"metadata_id" INT8 NOT NULL,
	"description" TEXT,
	"distance" FLOAT8,
	"duration" FLOAT8,
	"lat_starting_point_deg" FLOAT8,
	"lon_starting_point_deg" FLOAT8,
	"name" VARCHAR,
	"weather" JSONB,
	"created_at" TIMESTAMPTZ NOT NULL,
	FOREIGN KEY ("metadata_id") REFERENCES "metadata"("id")
);

CREATE TABLE "metadata_info"(
	"id" INT8 NOT NULL PRIMARY KEY,
	"metadata_id" INT8 NOT NULL,
	"data_spec_version" VARCHAR,
	"dataset_license" VARCHAR,
	"meta_data_spec_version" VARCHAR,
	"sequence_version" VARCHAR,
	"software_info" VARCHAR,
	"software_version" VARCHAR,
	"time_human" VARCHAR,
	"time_machine" FLOAT8,
	"created_at" TIMESTAMPTZ NOT NULL,
	FOREIGN KEY ("metadata_id") REFERENCES "metadata"("id")
);

CREATE TABLE "metadata_labeling"(
	"labeling_key" VARCHAR NOT NULL PRIMARY KEY,
	"metadata_id" INT8 NOT NULL,
	"creation_time_human" VARCHAR,
	"freetext" TEXT,
	"policy_version" VARCHAR,
	"provider" VARCHAR,
	"sensors_json" JSONB,
	FOREIGN KEY ("metadata_id") REFERENCES "metadata"("id")
);

CREATE TABLE "metadata_scenario"(
	"id" INT8 NOT NULL PRIMARY KEY,
	"metadata_id" INT8 NOT NULL,
	"environment_dynamics" VARCHAR,
	"environment_tags" JSONB,
	"name" VARCHAR,
	FOREIGN KEY ("metadata_id") REFERENCES "metadata"("id")
);

CREATE TABLE "metadata_sensor"(
	"metadata_id" INT8 NOT NULL,
	"sensor_key" VARCHAR NOT NULL PRIMARY KEY,
	"acquisition_rate" FLOAT8,
	"acquistion_mode" VARCHAR,
	"capture_rate" FLOAT8,
	"channel_number" INT4,
	"channel_space" VARCHAR,
	"firmware_version" VARCHAR,
	"focus_position_m" FLOAT8,
	"fov_horizontal_deg" FLOAT8,
	"fov_vertical_deg" FLOAT8,
	"frame_id" VARCHAR,
	"freetext" TEXT,
	"image_height" INT4,
	"image_width" INT4,
	"lens" VARCHAR,
	"manufacturer" VARCHAR,
	"max_exposure" INT4,
	"model" VARCHAR,
	"mtu" INT4,
	"optical_center_frame" VARCHAR,
	"ros_topics" JSONB,
	"sw_trigger_rate" FLOAT8,
	"time_stamp_accuracy" VARCHAR,
	"time_sync_method" VARCHAR,
	"trigger_method" VARCHAR,
	"trigger_mode" BOOL,
	"trigger_reference" VARCHAR,
	"trigger_source" VARCHAR,
	"type" VARCHAR,
	"created_at" TIMESTAMPTZ NOT NULL,
	FOREIGN KEY ("metadata_id") REFERENCES "metadata"("id")
);

CREATE TABLE "metadata_setup"(
	"id" INT8 NOT NULL PRIMARY KEY,
	"metadata_id" INT8 NOT NULL,
	"name" VARCHAR,
	"platform_description_link" VARCHAR,
	"created_at" TIMESTAMPTZ NOT NULL,
	FOREIGN KEY ("metadata_id") REFERENCES "metadata"("id")
);


CREATE TABLE "tags"(
	"id" INT8 NOT NULL PRIMARY KEY,
	"entry_id" INT8 NOT NULL,
	"name" VARCHAR NOT NULL,
	FOREIGN KEY ("entry_id") REFERENCES "entries"("id")
);

CREATE TABLE "topics"(
	"id" INT8 NOT NULL PRIMARY KEY,
	"entry_id" INT8 NOT NULL,
	"topic_name" VARCHAR NOT NULL,
	"message_count" INT8 NOT NULL,
	"type_description_hash" VARCHAR,
	"serialization_format" VARCHAR,
	"created_at" TIMESTAMPTZ NOT NULL,
	"type_" VARCHAR,
	FOREIGN KEY ("entry_id") REFERENCES "entries"("id")
);

DROP TABLE IF EXISTS "sensors";
