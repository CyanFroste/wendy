CREATE TABLE IF NOT EXISTS store (
    name        TEXT NOT NULL,
    key         TEXT NOT NULL,
    value       BLOB NOT NULL CHECK (json_valid(value, 8)),

    PRIMARY KEY (name, key)
) STRICT, WITHOUT ROWID;