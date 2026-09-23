CREATE TABLE jwt_blacklist (
    jti TEXT PRIMARY KEY,
    expires_at BIGINT NOT NULL,
    revoked_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_jwt_blacklist_expires_at
    ON jwt_blacklist (expires_at);