-- Your SQL goes here
CREATE TABLE "files"(
	"path" VARCHAR NOT NULL PRIMARY KEY,
	"last_modified" TIMESTAMP NOT NULL,
	"created" TIMESTAMP NOT NULL,
	"size" BIGINT NOT NULL,
	"last_checked" TIMESTAMP NOT NULL
);

