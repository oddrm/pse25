-- Your SQL goes here

ALTER TABLE "files" DROP COLUMN "is_generated_metadata";
ALTER TABLE "files" ADD COLUMN "is_custom_metadata" BOOL NOT NULL;





