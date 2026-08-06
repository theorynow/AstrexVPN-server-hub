CREATE TABLE nodes (
    id            VARCHAR(64) PRIMARY KEY,
    public_ip     VARCHAR(255) NOT NULL,
    name_en       VARCHAR(255) NOT NULL,
    country_code  VARCHAR(32) NOT NULL DEFAULT 'DE',
    country_flag  VARCHAR(32) NOT NULL,
    xray          JSONB,
    hysteria      JSONB,
    status        VARCHAR(32) NOT NULL DEFAULT 'offline',
    last_seen_at  TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    modified_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
