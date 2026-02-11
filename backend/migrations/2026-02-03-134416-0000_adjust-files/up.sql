-- Your SQL goes here

ALTER TABLE "files" DROP COLUMN "last_modified";
ALTER TABLE "files" DROP COLUMN "created";
ALTER TABLE "files" DROP COLUMN "size";
ALTER TABLE "files" DROP COLUMN "last_checked";
ALTER TABLE "files" ADD COLUMN "is_mcap" BOOL NOT NULL;
ALTER TABLE "files" ADD COLUMN "is_generated_metadata" BOOL NOT NULL;




ALTER TABLE "topics" DROP COLUMN "type";
ALTER TABLE "topics" ADD COLUMN "type_" VARCHAR;

