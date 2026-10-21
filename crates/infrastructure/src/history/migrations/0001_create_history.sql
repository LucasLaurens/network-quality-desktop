-- Never edit once released: add a new numbered migration instead.

CREATE TABLE networks (
    id INTEGER PRIMARY KEY,
    -- NULL while the gateway's MAC could not be read: such networks group by kind.
    gateway_mac BLOB,
    connection_kind TEXT NOT NULL,
    label TEXT,
    UNIQUE (gateway_mac, connection_kind)
);

-- One row per network, granularity (0 minute, 1 hour) and start (UTC seconds).
CREATE TABLE aggregates (
    network_id INTEGER NOT NULL REFERENCES networks (id),
    granularity INTEGER NOT NULL CHECK (granularity IN (0, 1)),
    start INTEGER NOT NULL,
    latency_hist BLOB NOT NULL,
    jitter_sum_us INTEGER NOT NULL,
    jitter_count INTEGER NOT NULL,
    probes_sent INTEGER NOT NULL,
    probes_lost INTEGER NOT NULL,
    s_solid INTEGER NOT NULL,
    s_unstable INTEGER NOT NULL,
    s_degraded INTEGER NOT NULL,
    s_offline INTEGER NOT NULL,
    PRIMARY KEY (network_id, granularity, start)
) WITHOUT ROWID;

CREATE TABLE events (
    ts INTEGER NOT NULL,
    network_id INTEGER NOT NULL REFERENCES networks (id),
    kind TEXT NOT NULL CHECK (kind IN ('network_changed', 'offline_started', 'offline_ended', 'sleep', 'wake'))
);
CREATE INDEX events_by_time ON events (ts);

CREATE TABLE throughput_tests (
    ts INTEGER NOT NULL,
    network_id INTEGER NOT NULL REFERENCES networks (id),
    profile TEXT NOT NULL,
    down_kbps INTEGER,
    up_kbps INTEGER,
    loaded_latency_ms INTEGER,
    bytes INTEGER NOT NULL,
    duration_ms INTEGER NOT NULL,
    status TEXT NOT NULL
);

-- A single typed row.
CREATE TABLE settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    autostart INTEGER NOT NULL DEFAULT 1,
    theme TEXT NOT NULL DEFAULT 'system' CHECK (theme IN ('system', 'light', 'dark')),
    ping_target TEXT,
    notify INTEGER NOT NULL DEFAULT 1,
    tests_allowed INTEGER NOT NULL DEFAULT 0
);
INSERT INTO settings (id) VALUES (1);
