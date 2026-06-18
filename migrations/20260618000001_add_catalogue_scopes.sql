-- Extend built-in roles with catalogue scopes.

UPDATE role_definitions
SET scopes = array(
    SELECT DISTINCT unnest(scopes || ARRAY[
        'catalogue:read',
        'catalogue:write'
    ])
)
WHERE name = 'user';

UPDATE role_definitions
SET scopes = array(
    SELECT DISTINCT unnest(scopes || ARRAY[
        'admin:catalogue'
    ])
)
WHERE name = 'admin';
