-- Create promocodes table
CREATE TABLE promocodes (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code                VARCHAR(16) NOT NULL UNIQUE,
    reward_type         VARCHAR(32) NOT NULL,
    reward_bytes        BIGINT NOT NULL,
    duration_days       INT NOT NULL DEFAULT 7,
    created_by_user_id  UUID REFERENCES users(id) ON DELETE SET NULL,
    used_by_user_id     UUID REFERENCES users(id) ON DELETE SET NULL,
    expires_at          TIMESTAMPTZ NOT NULL,
    used_at             TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_promocodes_code ON promocodes(code);
CREATE INDEX idx_promocodes_used_by ON promocodes(used_by_user_id);
