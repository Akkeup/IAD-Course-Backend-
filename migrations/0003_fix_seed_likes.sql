INSERT INTO andromeda_likes (user_id, star_id)
SELECT users.user_id, stars.star_id
FROM (
    VALUES
        ('andromeda_admin', 'J004047.84+405602.6'),
        ('andromeda_student', 'J004124.80+411634.7'),
        ('andromeda_admin', 'J004424.94+412322.3')
) AS seed(username, star_catalog_id)
JOIN andromeda_users AS users ON users.username = seed.username
JOIN andromeda_stars AS stars ON stars.star_catalog_id = seed.star_catalog_id
ON CONFLICT (user_id, star_id) DO NOTHING;