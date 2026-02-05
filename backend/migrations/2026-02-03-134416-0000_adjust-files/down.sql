-- This file should undo anything in `up.sql`

ALTER TABLE "files" DROP COLUMN "is_mcap";
ALTER TABLE "files" DROP COLUMN "is_generated_metadata";
ALTER TABLE "files" ADD COLUMN "last_modified" TIMESTAMP NOT NULL;
ALTER TABLE "files" ADD COLUMN "created" TIMESTAMP NOT NULL;
ALTER TABLE "files" ADD COLUMN "size" INT8 NOT NULL;
ALTER TABLE "files" ADD COLUMN "last_checked" TIMESTAMP NOT NULL;




ALTER TABLE "topics" DROP COLUMN "type_";
ALTER TABLE "topics" ADD COLUMN "type" VARCHAR;

