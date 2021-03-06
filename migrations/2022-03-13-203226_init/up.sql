CREATE TABLE pods (
    id INTEGER NOT NULL PRIMARY KEY,
    title VARCHAR NOT NULL,
    url VARCHAR NOT NULL,
    downloaded BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE episodes (
    id INTEGER NOT NULL PRIMARY KEY,
    uid VARCHAR NOT NULL,
    pod_id INTEGER NOT NULL,
    title VARCHAR NOT NULL,
    url VARCHAR NOT NULL,
    audio_url VARCHAR NOT NULL,
    description VARCHAR NOT NULL,
    audio_filepath VARCHAR,
    downloaded BOOLEAN NOT NULL DEFAULT FALSE,
    played BOOLEAN NOT NULL DEFAULT FALSE,
    timestamp REAL NOT NULL DEFAULT 0.0,
    pub_timestamp INTEGER NOT NULL,
    duration INTEGER
);
