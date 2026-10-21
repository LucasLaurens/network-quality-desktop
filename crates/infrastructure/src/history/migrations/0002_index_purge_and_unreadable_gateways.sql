-- Never edit once released: add a new numbered migration instead.

-- The purge looks for old minutes of every network: the primary key starts with the network.
CREATE INDEX aggregates_by_granularity ON aggregates (granularity, start);

-- UNIQUE treats NULL MACs as distinct: one row per kind for networks whose gateway is unreadable.
CREATE UNIQUE INDEX networks_without_gateway ON networks (connection_kind) WHERE gateway_mac IS NULL;
