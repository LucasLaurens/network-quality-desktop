-- Never edit once released: add a new numbered migration instead.

-- The dashboard reads one page of tests, newest first: the index walks only that page.
CREATE INDEX throughput_tests_by_time ON throughput_tests (ts);
