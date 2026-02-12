-- This file should undo anything in `up.sql`

ALTER TABLE "files" DROP COLUMN "is_custom_metadata";
ALTER TABLE "files" ADD COLUMN "is_generated_metadata" BOOL NOT NULL;





