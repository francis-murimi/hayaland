-- Extend built-in roles with party/deal scopes.
-- The array concatenation preserves existing scopes and adds new ones without duplicates.

UPDATE role_definitions
SET scopes = array(
    SELECT DISTINCT unnest(scopes || ARRAY[
        'parties:read',
        'parties:write',
        'deals:read',
        'deals:write',
        'deals:transition',
        'terms:negotiate',
        'payments:read',
        'payments:write'
    ])
)
WHERE name = 'user';

UPDATE role_definitions
SET scopes = array(
    SELECT DISTINCT unnest(scopes || ARRAY[
        'admin:parties',
        'admin:parties:read',
        'admin:parties:write',
        'admin:parties:delete',
        'admin:users',
        'admin:deals',
        'admin:*'
    ])
)
WHERE name = 'admin';
