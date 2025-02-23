CREATE TABLE file (
  id            INTEGER PRIMARY KEY,
  name          TEXT NOT NULL,
  content_type  TEXT,
  data          BLOB NOT NULL,
  modified      INTEGER NOT NULL DEFAULT (unixepoch())
) STRICT;
