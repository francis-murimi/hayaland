CREATE TABLE IF NOT EXISTS notifications (
    id UUID PRIMARY KEY,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    party_id UUID REFERENCES parties(id) ON DELETE CASCADE,
    notification_type TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    channels TEXT[] NOT NULL DEFAULT '{}',
    priority TEXT NOT NULL CHECK (priority IN ('LOW','NORMAL','HIGH','CRITICAL')),
    status TEXT NOT NULL DEFAULT 'PENDING' CHECK (status IN ('PENDING','SENT','DELIVERED','FAILED','SUPPRESSED')),
    read_at TIMESTAMPTZ,
    actioned_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    action_url TEXT,
    actions JSONB NOT NULL DEFAULT '[]',
    related_entity_type TEXT,
    related_entity_id UUID,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT notifications_recipient_check CHECK (
        (user_id IS NOT NULL) OR (party_id IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_notifications_user
    ON notifications(user_id, created_at DESC) WHERE user_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_notifications_party
    ON notifications(party_id, created_at DESC) WHERE party_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_notifications_status
    ON notifications(status) WHERE status IN ('PENDING','SENT');
CREATE INDEX IF NOT EXISTS idx_notifications_related
    ON notifications(related_entity_type, related_entity_id);

CREATE TABLE IF NOT EXISTS notification_preferences (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    channels JSONB NOT NULL DEFAULT '{"in_app":true,"email":true,"push":false,"sms":false}',
    per_type JSONB NOT NULL DEFAULT '{}',
    quiet_hours JSONB NOT NULL DEFAULT '{"enabled":false,"start":"22:00","end":"07:00","timezone":"UTC","except_critical":true}',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS notification_templates (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    notification_type TEXT NOT NULL,
    channel TEXT NOT NULL,
    locale TEXT NOT NULL DEFAULT 'en',
    subject_template TEXT NOT NULL,
    body_template TEXT NOT NULL,
    variables_schema JSONB NOT NULL DEFAULT '{}',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(notification_type, channel, locale)
);

CREATE INDEX IF NOT EXISTS idx_notification_templates_lookup
    ON notification_templates(notification_type, channel, locale) WHERE is_active = true;

CREATE TABLE IF NOT EXISTS notification_delivery_records (
    id UUID PRIMARY KEY,
    notification_id UUID NOT NULL REFERENCES notifications(id) ON DELETE CASCADE,
    channel TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('PENDING','SENT','DELIVERED','FAILED')),
    attempted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    delivered_at TIMESTAMPTZ,
    error_message TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    provider_reference TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_delivery_records_notification
    ON notification_delivery_records(notification_id, channel);

CREATE TABLE IF NOT EXISTS user_push_tokens (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_token TEXT NOT NULL,
    provider TEXT NOT NULL DEFAULT 'FCM',
    device_type TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at TIMESTAMPTZ,
    UNIQUE(user_id, device_token)
);

CREATE INDEX IF NOT EXISTS idx_user_push_tokens_user
    ON user_push_tokens(user_id);

-- Seed default English templates for common notification types.
INSERT INTO notification_templates (id, name, notification_type, channel, locale, subject_template, body_template, variables_schema)
VALUES
    (gen_random_uuid(), 'deal_invite_in_app', 'DEAL_INVITE', 'IN_APP', 'en', '', 'You have been invited to participate in the deal "{{deal_name}}" as {{role}}.', '{}')
ON CONFLICT (notification_type, channel, locale) DO NOTHING;

INSERT INTO notification_templates (id, name, notification_type, channel, locale, subject_template, body_template, variables_schema)
VALUES
    (gen_random_uuid(), 'deal_invite_email', 'DEAL_INVITE', 'EMAIL', 'en', 'Invitation to "{{deal_name}}"', 'Hi {{recipient_name}},\n\nYou have been invited to participate in the deal "{{deal_name}}" as {{role}}.\n\nReview the invitation: {{action_url}}', '{}')
ON CONFLICT (notification_type, channel, locale) DO NOTHING;

INSERT INTO notification_templates (id, name, notification_type, channel, locale, subject_template, body_template, variables_schema)
VALUES
    (gen_random_uuid(), 'deal_terms_locked_in_app', 'DEAL_TERMS_LOCKED', 'IN_APP', 'en', '', 'Terms have been locked for "{{deal_name}}". Please review and sign.', '{}')
ON CONFLICT (notification_type, channel, locale) DO NOTHING;

INSERT INTO notification_templates (id, name, notification_type, channel, locale, subject_template, body_template, variables_schema)
VALUES
    (gen_random_uuid(), 'deal_terms_locked_email', 'DEAL_TERMS_LOCKED', 'EMAIL', 'en', 'Terms locked for "{{deal_name}}"', 'Hi {{recipient_name}},\n\nThe terms for "{{deal_name}}" have been locked. Please review and sign by {{deadline}}.\n\n{{action_url}}', '{}')
ON CONFLICT (notification_type, channel, locale) DO NOTHING;

INSERT INTO notification_templates (id, name, notification_type, channel, locale, subject_template, body_template, variables_schema)
VALUES
    (gen_random_uuid(), 'milestone_verified_in_app', 'MILESTONE_VERIFIED', 'IN_APP', 'en', '', 'Milestone "{{milestone_title}}" for "{{deal_name}}" has been verified.', '{}')
ON CONFLICT (notification_type, channel, locale) DO NOTHING;

INSERT INTO notification_templates (id, name, notification_type, channel, locale, subject_template, body_template, variables_schema)
VALUES
    (gen_random_uuid(), 'dispute_opened_in_app', 'DISPUTE_OPENED', 'IN_APP', 'en', '', 'A dispute has been opened for "{{deal_name}}".', '{}')
ON CONFLICT (notification_type, channel, locale) DO NOTHING;

INSERT INTO notification_templates (id, name, notification_type, channel, locale, subject_template, body_template, variables_schema)
VALUES
    (gen_random_uuid(), 'admin_broadcast_in_app', 'ADMIN_BROADCAST', 'IN_APP', 'en', '', '{{body}}', '{}')
ON CONFLICT (notification_type, channel, locale) DO NOTHING;

INSERT INTO notification_templates (id, name, notification_type, channel, locale, subject_template, body_template, variables_schema)
VALUES
    (gen_random_uuid(), 'admin_broadcast_email', 'ADMIN_BROADCAST', 'EMAIL', 'en', '{{title}}', '{{body}}', '{}')
ON CONFLICT (notification_type, channel, locale) DO NOTHING;
