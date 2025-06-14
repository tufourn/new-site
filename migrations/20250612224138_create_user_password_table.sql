CREATE TABLE user_password (
    user_id uuid PRIMARY KEY,
    password_hash text NOT NULL,
    updated_at timestamptz DEFAULT NOW(),
    FOREIGN KEY (user_id) REFERENCES user_info (user_id) ON DELETE CASCADE
);

SELECT
    trigger_updated_at('user_password');
