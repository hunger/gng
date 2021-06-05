BEGIN;

CREATE TABLE IF NOT EXISTS repositories (
    uuid BLOB PRIMARY KEY NOT NULL CHECK(length(uuid) = 16),
    data BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS hashes (
    hash BLOB PRIMARY KEY NOT NULL CHECK(length(hash) = 65),
    kind INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS dependencies (
    hash BLOB PRIMARY KEY NOT NULL,
    dependency_kind INTEGER NOT NULL,
    dependency_hash BLOB NOT NULL,
    FOREIGN KEY(hash) REFERENCES hashes(hash) ON DELETE CASCADE,
    FOREIGN KEY(dependency_hash) REFERENCES hashes(hash) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS packet_data (
    hash BLOB PRIMARY KEY NOT NULL,
    data BLOB NOT NULL,
    FOREIGN KEY(hash) REFERENCES hashes(hash) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS known_facets (
    repository BLOB NOT NULL,
    name TEXT NOT NULL,
    defined_by BLOB NOT NULL,
    PRIMARY KEY(repository,name),
    FOREIGN KEY(repository) REFERENCES repositories(id) ON DELETE CASCADE,
    FOREIGN KEY(defined_by) REFERENCES hashes(hash) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS known_packets (
    repository BLOB NOT NULL,
    name TEXT NOT NULL,
    hash BLOB NOT NULL,
    PRIMARY KEY(repository,name),
    FOREIGN KEY(repository) REFERENCES repositories(id) ON DELETE CASCADE,
    FOREIGN KEY(hash) REFERENCES hashes(hash) ON DELETE CASCADE
);

PRAGMA user_version = 1;

COMMIT;
