UPDATE andromeda_users
SET password_hash = CASE username
    WHEN 'andromeda_admin' THEN '6ac11d4878f0f2323be651ba4141d7fc5c1c3c54d3ba98251e41e1440b50451d'
    WHEN 'andromeda_student' THEN '039eff277761a31f8b17e3a480da37105f69c9069c656f1f1ed04dc0c0026e41'
    ELSE password_hash
END
WHERE username IN ('andromeda_admin', 'andromeda_student');