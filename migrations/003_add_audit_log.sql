CREATE TABLE setting_revisions (
    id             UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    setting_key    TEXT        NOT NULL,
    action         TEXT        NOT NULL,
    previous_value JSONB,
    new_value      JSONB,
    changed_by     TEXT        NOT NULL,
    changed_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_revisions_key        ON setting_revisions (setting_key);
CREATE INDEX idx_revisions_changed_at ON setting_revisions (changed_at DESC);
CREATE INDEX idx_revisions_changed_by ON setting_revisions (changed_by);
CREATE INDEX idx_revisions_action     ON setting_revisions (action);
