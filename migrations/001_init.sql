CREATE TABLE relays(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT NOT NULL UNIQUE,
    last_check INTEGER DEFAULT NULL,            -- Last check as unix timestamp (seconds). NULL means never checked
    reachable BOOLEAN DEFAULT FALSE,            -- True if you can connect to the relay, otherwise false.
    nip11 TEXT DEFAULT NULL,                    -- NIP11 document. NULL means unavailable
    negentropy BOOLEAN DEFAULT NULL             -- Supports negentropy. NULL means unknown
);
