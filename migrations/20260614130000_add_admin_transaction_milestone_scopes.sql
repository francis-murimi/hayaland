-- Extend admin role with transaction and milestone oversight scopes.
UPDATE role_definitions
SET scopes = array(
    SELECT DISTINCT unnest(scopes || ARRAY['admin:transactions', 'admin:milestones'])
)
WHERE name = 'admin';
