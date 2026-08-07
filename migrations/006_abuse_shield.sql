CREATE TABLE device_identities (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    registered_with_platform text NOT NULL
        CHECK (registered_with_platform IN ('android', 'macos')),

    -- Server-side HMAC от platform-specific device material.
    device_key_hash bytea NOT NULL,

    trial_redeemed_at timestamptz NULL,

    UNIQUE (registered_with_platform, device_key_hash)
);

ALTER TABLE users ADD COLUMN device_identity_id uuid NULL REFERENCES device_identities(id);
