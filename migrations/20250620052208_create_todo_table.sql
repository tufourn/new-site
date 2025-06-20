CREATE TABLE todo (
    todo_id uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id uuid NOT NULL,
    todo_content text NOT NULL,
    is_completed boolean NOT NULL DEFAULT FALSE,
    created_at timestamptz DEFAULT NOW(),
    updated_at timestamptz DEFAULT NOW(),
    FOREIGN KEY (user_id) REFERENCES user_info (user_id) ON DELETE CASCADE
);

SELECT
    trigger_updated_at('todo');
