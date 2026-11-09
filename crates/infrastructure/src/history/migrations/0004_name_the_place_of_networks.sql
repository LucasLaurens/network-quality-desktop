-- Never edit once released: add a new numbered migration instead.

-- What the user calls the place a network serves; empty until named. The writer never touches it.
ALTER TABLE networks ADD COLUMN place TEXT NOT NULL DEFAULT '';
