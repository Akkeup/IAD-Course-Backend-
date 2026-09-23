CREATE TABLE andromeda_users (
    user_id BIGSERIAL PRIMARY KEY,
    username VARCHAR(100) NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE andromeda_stars (
    star_id BIGSERIAL PRIMARY KEY,
    created_by BIGINT NOT NULL REFERENCES andromeda_users(user_id),
    star_name VARCHAR(150) NOT NULL,
    star_catalog_id VARCHAR(150) NOT NULL UNIQUE,
    distance_kpc NUMERIC(6, 2) NOT NULL CHECK (distance_kpc >= 0),
    velocity_kms INTEGER NOT NULL CHECK (velocity_kms >= 0),
    star_status VARCHAR(20) NOT NULL DEFAULT 'draft'
        CHECK (star_status IN ('draft', 'published', 'deleted')),
    star_description TEXT NOT NULL DEFAULT '',
    image_url TEXT NOT NULL DEFAULT '',
    video_url TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE andromeda_likes (
    user_id BIGINT NOT NULL REFERENCES andromeda_users(user_id),
    star_id BIGINT NOT NULL REFERENCES andromeda_stars(star_id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, star_id)
);

CREATE INDEX idx_andromeda_stars_status_distance
    ON andromeda_stars (star_status, distance_kpc);

CREATE INDEX idx_andromeda_likes_star_id
    ON andromeda_likes (star_id);

CREATE UNIQUE INDEX one_draft_per_user
    ON andromeda_stars (created_by)
    WHERE star_status = 'draft';