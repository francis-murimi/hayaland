-- Add notification scopes to built-in roles.

UPDATE role_definitions
SET scopes = array_append(scopes, 'notifications:read')
WHERE name = 'user'
  AND NOT ('notifications:read' = ANY(scopes));

UPDATE role_definitions
SET scopes = array_append(scopes, 'notifications:write')
WHERE name = 'user'
  AND NOT ('notifications:write' = ANY(scopes));

UPDATE role_definitions
SET scopes = array_append(scopes, 'admin:notifications')
WHERE name = 'admin'
  AND NOT ('admin:notifications' = ANY(scopes));
