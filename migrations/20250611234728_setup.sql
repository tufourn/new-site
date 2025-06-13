-- taken from realworld-axum-sqlx
CREATE extension IF NOT EXISTS "uuid-ossp";

CREATE
OR REPLACE FUNCTION set_updated_at() RETURNS trigger AS
$$
BEGIN
NEW.updated_at = NOW();

RETURN NEW;

END;

$$
language plpgsql;

CREATE
OR REPLACE FUNCTION trigger_updated_at(tablename regclass) RETURNS void AS
$$
BEGIN
EXECUTE format(
    'CREATE TRIGGER set_updated_at
        BEFORE UPDATE
        ON %s
        FOR EACH ROW
        WHEN (OLD is distinct from NEW)
    EXECUTE FUNCTION set_updated_at();',
    tablename
);

END;

$$
language plpgsql;

CREATE COLLATION case_insensitive (
    provider = icu,
    locale = 'und-u-ks-level2',
    DETERMINISTIC = false
);
