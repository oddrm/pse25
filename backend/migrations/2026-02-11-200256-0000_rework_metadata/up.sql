-- Your SQL goes here
ALTER TABLE "entries" DROP COLUMN "platform";
ALTER TABLE "entries" DROP COLUMN "start_time_ns";
ALTER TABLE "entries" DROP COLUMN "duration_ns";
ALTER TABLE "entries" DROP COLUMN "total_message_count";
ALTER TABLE "entries" DROP COLUMN "storage_identifier";
ALTER TABLE "entries" DROP COLUMN "compression_format";
ALTER TABLE "entries" DROP COLUMN "compression_mode";
ALTER TABLE "entries" ADD COLUMN "sequence_lon_starting_point_deg" BIGSERIAL;
ALTER TABLE "entries" ADD COLUMN "scenario_name" VARCHAR;
ALTER TABLE "entries" ADD COLUMN "scenario_description" TEXT;
ALTER TABLE "entries" ADD COLUMN "sequence_lat_starting_point_deg" BIGSERIAL;
ALTER TABLE "entries" ADD COLUMN "tags" TEXT[] NOT NULL;
ALTER TABLE "entries" ADD COLUMN "sequence_duration" BIGSERIAL;
ALTER TABLE "entries" ADD COLUMN "weather_road_humidity" VARCHAR;
ALTER TABLE "entries" ADD COLUMN "time_machine" BIGINT;
ALTER TABLE "entries" ADD COLUMN "platform_name" VARCHAR;
ALTER TABLE "entries" ADD COLUMN "scenario_creation_time" TIMESTAMPTZ;
ALTER TABLE "entries" ADD COLUMN "weather_cloudiness" VARCHAR;
ALTER TABLE "entries" ADD COLUMN "weather_precipitation" VARCHAR;
ALTER TABLE "entries" ADD COLUMN "weather_wind_intensity" VARCHAR;
ALTER TABLE "entries" ADD COLUMN "weather_fog" BOOLEAN;
ALTER TABLE "entries" ADD COLUMN "weather_precipitation_deposits" VARCHAR;
ALTER TABLE "entries" ADD COLUMN "platform_image_link" VARCHAR;
ALTER TABLE "entries" ADD COLUMN "weather_snow" BOOLEAN;
ALTER TABLE "entries" ADD COLUMN "sequence_distance" BIGSERIAL;
ALTER TABLE "entries" ADD COLUMN "topics" TEXT[] NOT NULL;

DROP TABLE IF EXISTS "metadata_dataset_sequence";
DROP TABLE IF EXISTS "metadata_info";
DROP TABLE IF EXISTS "metadata_labeling";
DROP TABLE IF EXISTS "metadata_scenario";
DROP TABLE IF EXISTS "metadata_sensor";
DROP TABLE IF EXISTS "metadata_setup";
DROP TABLE IF EXISTS "metadata";

DROP TABLE IF EXISTS "tags";
DROP TABLE IF EXISTS "topics";
CREATE TABLE "sensors"(
	"id" BIGINT NOT NULL PRIMARY KEY,
	"entry_id" BIGINT NOT NULL,
	"sensor_name" VARCHAR NOT NULL,
	"manufacturer" VARCHAR,
	"sensor_type" VARCHAR,
	"ros_topics" TEXT[] NOT NULL,
	"custom_parameters" JSONB,
	FOREIGN KEY ("entry_id") REFERENCES "entries"("id")
);

CREATE INDEX IF NOT EXISTS idx_entries_tags ON entries USING GIN (tags);

CREATE INDEX IF NOT EXISTS idx_entries_topics ON entries USING GIN (topics);