CREATE TABLE settings (
    id           UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    key          TEXT         NOT NULL UNIQUE,
    setting_type TEXT         NOT NULL,
    value        JSONB        NOT NULL,
    description  TEXT,
    is_active    BOOLEAN      NOT NULL DEFAULT true,
    created_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_settings_key       ON settings (key);
CREATE INDEX idx_settings_type      ON settings (setting_type);
CREATE INDEX idx_settings_value_gin ON settings USING GIN (value);
