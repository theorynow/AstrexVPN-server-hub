-- Create promocodes table
CREATE TABLE promocodes (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code                VARCHAR(16) NOT NULL UNIQUE,
    reward_type         VARCHAR(32) NOT NULL,
    reward_bytes        BIGINT NOT NULL,
    duration_days       INT NOT NULL DEFAULT 7,
    max_uses            INT NOT NULL DEFAULT 1,
    current_uses        INT NOT NULL DEFAULT 0,
    created_by_user_id  UUID REFERENCES users(id) ON DELETE SET NULL,
    expires_at          TIMESTAMPTZ NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Create promocode_redemptions table for tracking activations per user
CREATE TABLE promocode_redemptions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    promocode_id    UUID NOT NULL REFERENCES promocodes(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    redeemed_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (promocode_id, user_id)
);

CREATE INDEX idx_promocodes_code ON promocodes(code);
CREATE INDEX idx_promocode_redemptions_user ON promocode_redemptions(user_id);
CREATE INDEX idx_promocode_redemptions_promo ON promocode_redemptions(promocode_id);
