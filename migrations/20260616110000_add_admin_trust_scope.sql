-- Add admin:trust scope to the built-in admin role.

UPDATE role_definitions
SET scopes = array_append(scopes, 'admin:trust')
WHERE name = 'admin'
  AND NOT ('admin:trust' = ANY(scopes));
