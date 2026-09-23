UPDATE andromeda_users
SET password_hash = CASE username
    WHEN 'andromeda_admin' THEN '8c6976e5b5410415bde908bd4dee15dfb167a9c873fc4bb8a81f6f2ab448a918'
    WHEN 'andromeda_student' THEN '264c8c381bf16c982a4e59b0dd4c6f7808c51a05f64c35db42cc78a2a72875bb'
    ELSE password_hash
END
WHERE username IN ('andromeda_admin', 'andromeda_student');
