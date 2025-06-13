CREATE TABLE "password" (
    user_id uuid PRIMARY KEY,
    password_hash text NOT NULL,
    updated_at timestamptz NOT NULL,
    FOREIGN KEY (user_id) REFERENCES "user" (user_id) ON DELETE CASCADE
);

SELECT
    trigger_updated_at('"password"');
