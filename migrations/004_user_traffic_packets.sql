-- Create user_traffic_packets table
CREATE TABLE user_traffic_packets (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id                 UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    traffic_limit_bytes     BIGINT NOT NULL,
    traffic_remaining_bytes BIGINT NOT NULL,
    expires_at              TIMESTAMPTZ NOT NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    modified_at             TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Migrate existing traffic for users into user_traffic_packets
INSERT INTO user_traffic_packets (user_id, traffic_limit_bytes, traffic_remaining_bytes, expires_at)
SELECT id, traffic_limit_bytes, GREATEST(0, traffic_limit_bytes - traffic_used_bytes), now() + INTERVAL '30 days'
FROM users;

-- Drop deprecated traffic columns on users
ALTER TABLE users DROP COLUMN traffic_limit_bytes;
ALTER TABLE users DROP COLUMN traffic_used_bytes;
