-- Create "repositoires" Table
CREATE TABLE repositories (
    id INTEGER PRIMARY KEY NOT NULL,
    uuid BLOB NOT NULL CHECK(length(uuid) = 16)
);
