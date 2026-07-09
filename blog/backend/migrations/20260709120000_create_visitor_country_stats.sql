CREATE TABLE IF NOT EXISTS visitor_country_stats (
    day TEXT NOT NULL,
    country_code TEXT NOT NULL,
    path_group TEXT NOT NULL,
    visit_count INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (day, country_code, path_group)
);

CREATE INDEX IF NOT EXISTS idx_visitor_country_stats_day
ON visitor_country_stats(day);
