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
