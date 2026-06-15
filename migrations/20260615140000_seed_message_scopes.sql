-- Seed messaging-related scopes into role definitions.
UPDATE role_definitions
SET scopes = array(
    SELECT DISTINCT unnest(scopes || ARRAY['messages:read', 'messages:write'])
)
WHERE name = 'user';

UPDATE role_definitions
SET scopes = array(
    SELECT DISTINCT unnest(scopes || ARRAY[
        'chatrooms:read',
        'chatrooms:write',
        'chatrooms:moderate',
        'admin:messages'
    ])
)
WHERE name = 'admin';
