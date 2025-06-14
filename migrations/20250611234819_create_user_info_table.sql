CREATE TABLE user_info (
    user_id uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    username text COLLATE "case_insensitive" UNIQUE NOT NULL,
    email text COLLATE "case_insensitive" UNIQUE NOT NULL,
    created_at timestamptz NOT NULL DEFAULT NOW(),
    updated_at timestamptz
);

SELECT
    trigger_updated_at('user_info');
