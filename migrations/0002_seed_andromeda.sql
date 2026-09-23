INSERT INTO andromeda_users (username, password_hash)
VALUES
    ('andromeda_admin', '$2b$12$N9qo8uLOickgx2ZMRZoMyeIjZAgcfl7p92ldGxad68LJZdL17lhWy'),
    ('andromeda_student', '$2b$12$N9qo8uLOickgx2ZMRZoMyeIjZAgcfl7p92ldGxad68LJZdL17lhWy')
ON CONFLICT (username) DO NOTHING;

INSERT INTO andromeda_stars (
    created_by,
    star_name,
    star_catalog_id,
    distance_kpc,
    velocity_kms,
    star_status,
    star_description,
    image_url,
    video_url
)
SELECT
    users.user_id,
    seed.star_name,
    seed.star_catalog_id,
    seed.distance_kpc,
    seed.velocity_kms,
    seed.star_status,
    seed.star_description,
    seed.image_url,
    seed.video_url
FROM (
    VALUES
        ('andromeda_admin', 'M31-V1', 'M31-V1', 18.40::NUMERIC, 225, 'published', 'Переменная звезда типа цефеиды в галактике Андромеды.', 'images/m31-v1.webp', 'videos/m31-v1.mp4'),
        ('andromeda_admin', 'M31-1775', 'J004047.84+405602.6', 18.40::NUMERIC, 225, 'published', 'Красный сверхгигант с необычно сильным покраснением спектра.', 'images/m31-1775.webp', 'videos/m31-1775.mp4'),
        ('andromeda_admin', 'M31-1515', 'J004124.80+411634.7', 8.70::NUMERIC, 220, 'published', 'Красный сверхгигант спектрального класса M3 I.', 'images/m31-1515.webp', 'videos/m31-1515.mp4'),
        ('andromeda_admin', 'M31-2252', 'J004424.94+412322.3', 22.60::NUMERIC, 220, 'published', 'Яркий красный сверхгигант в диске M31.', 'images/m31-2252.webp', 'videos/m31-2252.mp4'),
        ('andromeda_admin', 'M31-1372', 'J004454.38+412441.6', 6.30::NUMERIC, 218, 'published', 'Красный сверхгигант из выборки звёзд Андромеды.', 'images/m31-1372.webp', 'videos/m31-1372.mp4'),
        ('andromeda_admin', 'M31-504', 'J004447.08+412801.7', 25.00::NUMERIC, 220, 'published', 'Красный сверхгигант у внешней границы диска M31.', 'images/m31-504.webp', 'videos/m31-504.mp4'),
        ('andromeda_student', 'M31-DRAFT', 'M31-DRAFT', 12.50::NUMERIC, 223, 'draft', 'Черновик карточки для проверки сценария публикации.', '', ''),
        ('andromeda_student', 'M31-DELETED', 'M31-DELETED', 33.50::NUMERIC, 185, 'deleted', 'Логически удалённая карточка.', 'images/m31-1494.webp', 'videos/m31-1494.mp4')
) AS seed(username, star_name, star_catalog_id, distance_kpc, velocity_kms, star_status, star_description, image_url, video_url)
JOIN andromeda_users AS users ON users.username = seed.username
ON CONFLICT (star_catalog_id) DO NOTHING;

INSERT INTO andromeda_likes (user_id, star_id)
SELECT users.user_id, stars.star_id
FROM (
    VALUES
        ('andromeda_admin', 'M31-V1'),
        ('andromeda_student', 'M31-V1'),
        ('andromeda_admin', 'M31-1775'),
        ('andromeda_student', 'M31-1515'),
        ('andromeda_admin', 'M31-2252')
) AS seed(username, star_catalog_id)
JOIN andromeda_users AS users ON users.username = seed.username
JOIN andromeda_stars AS stars ON stars.star_catalog_id = seed.star_catalog_id
ON CONFLICT (user_id, star_id) DO NOTHING;