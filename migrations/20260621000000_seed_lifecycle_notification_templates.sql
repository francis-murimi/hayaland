-- Seed default English templates for lifecycle notification types.
-- These templates are rendered by SendNotification when business flows emit events.
-- All inserts are idempotent (ON CONFLICT DO NOTHING).

-- Deal lifecycle
INSERT INTO notification_templates (id, name, notification_type, channel, locale, subject_template, body_template, variables_schema)
VALUES
    (gen_random_uuid(), 'deal_submitted_in_app', 'DEAL_SUBMITTED', 'IN_APP', 'en', '', 'The deal "{{deal_name}}" has been submitted for review.', '{"deal_name": "string", "deal_id": "uuid"}'),
    (gen_random_uuid(), 'deal_submitted_email', 'DEAL_SUBMITTED', 'EMAIL', 'en', 'Deal "{{deal_name}}" submitted', 'Hi {{recipient_name}},

The deal "{{deal_name}}" has been submitted for review.

View deal: {{action_url}}', '{"deal_name": "string", "deal_id": "uuid", "recipient_name": "string"}'),

    (gen_random_uuid(), 'deal_terms_locked_in_app', 'DEAL_TERMS_LOCKED', 'IN_APP', 'en', '', 'Terms have been locked for "{{deal_name}}". Please review and sign.', '{"deal_name": "string", "deal_id": "uuid"}'),
    (gen_random_uuid(), 'deal_terms_locked_email', 'DEAL_TERMS_LOCKED', 'EMAIL', 'en', 'Terms locked for "{{deal_name}}"', 'Hi {{recipient_name}},

The terms for "{{deal_name}}" have been locked. Please review and sign.

View deal: {{action_url}}', '{"deal_name": "string", "deal_id": "uuid", "recipient_name": "string"}'),

    (gen_random_uuid(), 'deal_committed_in_app', 'DEAL_COMMITTED', 'IN_APP', 'en', '', '"{{deal_name}}" has been committed. Execution can begin once milestones are ready.', '{"deal_name": "string", "deal_id": "uuid"}'),
    (gen_random_uuid(), 'deal_committed_email', 'DEAL_COMMITTED', 'EMAIL', 'en', '"{{deal_name}}" committed', 'Hi {{recipient_name}},

"{{deal_name}}" has been committed by all parties. Execution can begin once milestones are ready.

View deal: {{action_url}}', '{"deal_name": "string", "deal_id": "uuid", "recipient_name": "string"}'),

    (gen_random_uuid(), 'deal_executing_in_app', 'DEAL_EXECUTING', 'IN_APP', 'en', '', '"{{deal_name}}" is now executing.', '{"deal_name": "string", "deal_id": "uuid"}'),
    (gen_random_uuid(), 'deal_executing_email', 'DEAL_EXECUTING', 'EMAIL', 'en', '"{{deal_name}}" is executing', 'Hi {{recipient_name}},

"{{deal_name}}" has moved to the executing state.

View deal: {{action_url}}', '{"deal_name": "string", "deal_id": "uuid", "recipient_name": "string"}'),

    (gen_random_uuid(), 'deal_completed_in_app', 'DEAL_COMPLETED', 'IN_APP', 'en', '', '"{{deal_name}}" has been completed.', '{"deal_name": "string", "deal_id": "uuid"}'),
    (gen_random_uuid(), 'deal_completed_email', 'DEAL_COMPLETED', 'EMAIL', 'en', '"{{deal_name}}" completed', 'Hi {{recipient_name}},

"{{deal_name}}" has been completed. Please submit your reviews.

View deal: {{action_url}}', '{"deal_name": "string", "deal_id": "uuid", "recipient_name": "string"}'),

    (gen_random_uuid(), 'deal_cancelled_in_app', 'DEAL_CANCELLED', 'IN_APP', 'en', '', '"{{deal_name}}" has been cancelled.', '{"deal_name": "string", "deal_id": "uuid"}'),
    (gen_random_uuid(), 'deal_cancelled_email', 'DEAL_CANCELLED', 'EMAIL', 'en', '"{{deal_name}}" cancelled', 'Hi {{recipient_name}},

"{{deal_name}}" has been cancelled.

View deal: {{action_url}}', '{"deal_name": "string", "deal_id": "uuid", "recipient_name": "string"}'),

    (gen_random_uuid(), 'deal_expired_in_app', 'DEAL_EXPIRED', 'IN_APP', 'en', '', '"{{deal_name}}" has expired due to inactivity.', '{"deal_name": "string", "deal_id": "uuid"}'),
    (gen_random_uuid(), 'deal_expired_email', 'DEAL_EXPIRED', 'EMAIL', 'en', '"{{deal_name}}" expired', 'Hi {{recipient_name}},

"{{deal_name}}" has expired due to inactivity.

View deal: {{action_url}}', '{"deal_name": "string", "deal_id": "uuid", "recipient_name": "string"}'),

    (gen_random_uuid(), 'deal_disputed_in_app', 'DEAL_DISPUTED', 'IN_APP', 'en', '', 'A dispute has been opened for "{{deal_name}}".', '{"deal_name": "string", "deal_id": "uuid"}'),
    (gen_random_uuid(), 'deal_disputed_email', 'DEAL_DISPUTED', 'EMAIL', 'en', 'Dispute opened for "{{deal_name}}"', 'Hi {{recipient_name}},

A dispute has been opened for "{{deal_name}}". An admin will review the case.

View deal: {{action_url}}', '{"deal_name": "string", "deal_id": "uuid", "recipient_name": "string"}')
ON CONFLICT (notification_type, channel, locale) DO NOTHING;

-- Negotiation
INSERT INTO notification_templates (id, name, notification_type, channel, locale, subject_template, body_template, variables_schema)
VALUES
    (gen_random_uuid(), 'term_proposed_in_app', 'TERM_PROPOSED', 'IN_APP', 'en', '', 'A new term "{{term_name}}" was proposed on "{{deal_name}}".', '{"term_name": "string", "deal_name": "string", "deal_id": "uuid"}'),
    (gen_random_uuid(), 'term_proposed_email', 'TERM_PROPOSED', 'EMAIL', 'en', 'New term on "{{deal_name}}"', 'Hi {{recipient_name}},

A new term "{{term_name}}" was proposed on "{{deal_name}}".

View deal: {{action_url}}', '{"term_name": "string", "deal_name": "string", "deal_id": "uuid", "recipient_name": "string"}'),

    (gen_random_uuid(), 'term_accepted_in_app', 'TERM_ACCEPTED', 'IN_APP', 'en', '', 'The term "{{term_name}}" on "{{deal_name}}" was accepted.', '{"term_name": "string", "deal_name": "string", "deal_id": "uuid"}'),
    (gen_random_uuid(), 'term_accepted_email', 'TERM_ACCEPTED', 'EMAIL', 'en', 'Term accepted on "{{deal_name}}"', 'Hi {{recipient_name}},

The term "{{term_name}}" on "{{deal_name}}" was accepted.

View deal: {{action_url}}', '{"term_name": "string", "deal_name": "string", "deal_id": "uuid", "recipient_name": "string"}'),

    (gen_random_uuid(), 'term_rejected_in_app', 'TERM_REJECTED', 'IN_APP', 'en', '', 'The term "{{term_name}}" on "{{deal_name}}" was rejected.', '{"term_name": "string", "deal_name": "string", "deal_id": "uuid"}'),
    (gen_random_uuid(), 'term_rejected_email', 'TERM_REJECTED', 'EMAIL', 'en', 'Term rejected on "{{deal_name}}"', 'Hi {{recipient_name}},

The term "{{term_name}}" on "{{deal_name}}" was rejected.

View deal: {{action_url}}', '{"term_name": "string", "deal_name": "string", "deal_id": "uuid", "recipient_name": "string"}'),

    (gen_random_uuid(), 'term_countered_in_app', 'TERM_COUNTERED', 'IN_APP', 'en', '', 'The term "{{term_name}}" on "{{deal_name}}" was countered.', '{"term_name": "string", "deal_name": "string", "deal_id": "uuid"}'),
    (gen_random_uuid(), 'term_countered_email', 'TERM_COUNTERED', 'EMAIL', 'en', 'Term countered on "{{deal_name}}"', 'Hi {{recipient_name}},

The term "{{term_name}}" on "{{deal_name}}" was countered.

View deal: {{action_url}}', '{"term_name": "string", "deal_name": "string", "deal_id": "uuid", "recipient_name": "string"}')
ON CONFLICT (notification_type, channel, locale) DO NOTHING;

-- Milestones
INSERT INTO notification_templates (id, name, notification_type, channel, locale, subject_template, body_template, variables_schema)
VALUES
    (gen_random_uuid(), 'milestone_assigned_in_app', 'MILESTONE_ASSIGNED', 'IN_APP', 'en', '', 'A milestone "{{milestone_title}}" was assigned to your party on "{{deal_name}}".', '{"milestone_title": "string", "deal_name": "string", "deal_id": "uuid"}'),
    (gen_random_uuid(), 'milestone_assigned_email', 'MILESTONE_ASSIGNED', 'EMAIL', 'en', 'Milestone assigned on "{{deal_name}}"', 'Hi {{recipient_name}},

A milestone "{{milestone_title}}" was assigned to your party on "{{deal_name}}".

View deal: {{action_url}}', '{"milestone_title": "string", "deal_name": "string", "deal_id": "uuid", "recipient_name": "string"}'),

    (gen_random_uuid(), 'milestone_started_in_app', 'MILESTONE_STARTED', 'IN_APP', 'en', '', 'The milestone "{{milestone_title}}" on "{{deal_name}}" has started.', '{"milestone_title": "string", "deal_name": "string", "deal_id": "uuid"}'),
    (gen_random_uuid(), 'milestone_started_email', 'MILESTONE_STARTED', 'EMAIL', 'en', 'Milestone started on "{{deal_name}}"', 'Hi {{recipient_name}},

The milestone "{{milestone_title}}" on "{{deal_name}}" has started.

View deal: {{action_url}}', '{"milestone_title": "string", "deal_name": "string", "deal_id": "uuid", "recipient_name": "string"}'),

    (gen_random_uuid(), 'milestone_completed_in_app', 'MILESTONE_COMPLETED', 'IN_APP', 'en', '', 'The milestone "{{milestone_title}}" on "{{deal_name}}" has been marked complete.', '{"milestone_title": "string", "deal_name": "string", "deal_id": "uuid"}'),
    (gen_random_uuid(), 'milestone_completed_email', 'MILESTONE_COMPLETED', 'EMAIL', 'en', 'Milestone completed on "{{deal_name}}"', 'Hi {{recipient_name}},

The milestone "{{milestone_title}}" on "{{deal_name}}" has been marked complete.

View deal: {{action_url}}', '{"milestone_title": "string", "deal_name": "string", "deal_id": "uuid", "recipient_name": "string"}'),

    (gen_random_uuid(), 'milestone_verified_in_app', 'MILESTONE_VERIFIED', 'IN_APP', 'en', '', 'The milestone "{{milestone_title}}" on "{{deal_name}}" has been verified.', '{"milestone_title": "string", "deal_name": "string", "deal_id": "uuid"}'),
    (gen_random_uuid(), 'milestone_verified_email', 'MILESTONE_VERIFIED', 'EMAIL', 'en', 'Milestone verified on "{{deal_name}}"', 'Hi {{recipient_name}},

The milestone "{{milestone_title}}" on "{{deal_name}}" has been verified.

View deal: {{action_url}}', '{"milestone_title": "string", "deal_name": "string", "deal_id": "uuid", "recipient_name": "string"}'),

    (gen_random_uuid(), 'milestone_due_in_app', 'MILESTONE_DUE', 'IN_APP', 'en', '', 'The milestone "{{milestone_title}}" on "{{deal_name}}" is due soon.', '{"milestone_title": "string", "deal_name": "string", "deal_id": "uuid"}'),
    (gen_random_uuid(), 'milestone_due_email', 'MILESTONE_DUE', 'EMAIL', 'en', 'Milestone due on "{{deal_name}}"', 'Hi {{recipient_name}},

The milestone "{{milestone_title}}" on "{{deal_name}}" is due soon.

View deal: {{action_url}}', '{"milestone_title": "string", "deal_name": "string", "deal_id": "uuid", "recipient_name": "string"}')
ON CONFLICT (notification_type, channel, locale) DO NOTHING;

-- Payments
INSERT INTO notification_templates (id, name, notification_type, channel, locale, subject_template, body_template, variables_schema)
VALUES
    (gen_random_uuid(), 'escrow_funded_in_app', 'ESCROW_FUNDED', 'IN_APP', 'en', '', 'Escrow of {{amount}} has been funded for "{{deal_name}}".', '{"amount": "string", "deal_name": "string", "deal_id": "uuid"}'),
    (gen_random_uuid(), 'escrow_funded_email', 'ESCROW_FUNDED', 'EMAIL', 'en', 'Escrow funded for "{{deal_name}}"', 'Hi {{recipient_name}},

Escrow of {{amount}} has been funded for "{{deal_name}}".

View deal: {{action_url}}', '{"amount": "string", "deal_name": "string", "deal_id": "uuid", "recipient_name": "string"}'),

    (gen_random_uuid(), 'escrow_released_in_app', 'ESCROW_RELEASED', 'IN_APP', 'en', '', 'Escrow of {{amount}} has been released for "{{milestone_title}}" on "{{deal_name}}".', '{"amount": "string", "milestone_title": "string", "deal_name": "string", "deal_id": "uuid"}'),
    (gen_random_uuid(), 'escrow_released_email', 'ESCROW_RELEASED', 'EMAIL', 'en', 'Escrow released for "{{deal_name}}"', 'Hi {{recipient_name}},

Escrow of {{amount}} has been released for "{{milestone_title}}" on "{{deal_name}}".

View deal: {{action_url}}', '{"amount": "string", "milestone_title": "string", "deal_name": "string", "deal_id": "uuid", "recipient_name": "string"}'),

    (gen_random_uuid(), 'payment_due_in_app', 'PAYMENT_DUE', 'IN_APP', 'en', '', 'A payment of {{amount}} is due for "{{deal_name}}".', '{"amount": "string", "deal_name": "string", "deal_id": "uuid"}'),
    (gen_random_uuid(), 'payment_due_email', 'PAYMENT_DUE', 'EMAIL', 'en', 'Payment due for "{{deal_name}}"', 'Hi {{recipient_name}},

A payment of {{amount}} is due for "{{deal_name}}".

View deal: {{action_url}}', '{"amount": "string", "deal_name": "string", "deal_id": "uuid", "recipient_name": "string"}'),

    (gen_random_uuid(), 'payment_received_in_app', 'PAYMENT_RECEIVED', 'IN_APP', 'en', '', 'A payment of {{amount}} has been received for "{{deal_name}}".', '{"amount": "string", "deal_name": "string", "deal_id": "uuid"}'),
    (gen_random_uuid(), 'payment_received_email', 'PAYMENT_RECEIVED', 'EMAIL', 'en', 'Payment received for "{{deal_name}}"', 'Hi {{recipient_name}},

A payment of {{amount}} has been received for "{{deal_name}}".

View deal: {{action_url}}', '{"amount": "string", "deal_name": "string", "deal_id": "uuid", "recipient_name": "string"}'),

    (gen_random_uuid(), 'transaction_pending_approval_in_app', 'TRANSACTION_PENDING_APPROVAL', 'IN_APP', 'en', '', 'A transaction of {{amount}} is pending your approval for "{{deal_name}}".', '{"amount": "string", "deal_name": "string", "deal_id": "uuid"}'),
    (gen_random_uuid(), 'transaction_pending_approval_email', 'TRANSACTION_PENDING_APPROVAL', 'EMAIL', 'en', 'Transaction pending approval for "{{deal_name}}"', 'Hi {{recipient_name}},

A transaction of {{amount}} is pending your approval for "{{deal_name}}".

View deal: {{action_url}}', '{"amount": "string", "deal_name": "string", "deal_id": "uuid", "recipient_name": "string"}'),

    (gen_random_uuid(), 'transaction_approved_in_app', 'TRANSACTION_APPROVED', 'IN_APP', 'en', '', 'A transaction of {{amount}} has been approved for "{{deal_name}}".', '{"amount": "string", "deal_name": "string", "deal_id": "uuid"}'),
    (gen_random_uuid(), 'transaction_approved_email', 'TRANSACTION_APPROVED', 'EMAIL', 'en', 'Transaction approved for "{{deal_name}}"', 'Hi {{recipient_name}},

A transaction of {{amount}} has been approved for "{{deal_name}}".

View deal: {{action_url}}', '{"amount": "string", "deal_name": "string", "deal_id": "uuid", "recipient_name": "string"}'),

    (gen_random_uuid(), 'transaction_rejected_in_app', 'TRANSACTION_REJECTED', 'IN_APP', 'en', '', 'A transaction of {{amount}} has been rejected for "{{deal_name}}".', '{"amount": "string", "deal_name": "string", "deal_id": "uuid"}'),
    (gen_random_uuid(), 'transaction_rejected_email', 'TRANSACTION_REJECTED', 'EMAIL', 'en', 'Transaction rejected for "{{deal_name}}"', 'Hi {{recipient_name}},

A transaction of {{amount}} has been rejected for "{{deal_name}}".

View deal: {{action_url}}', '{"amount": "string", "deal_name": "string", "deal_id": "uuid", "recipient_name": "string"}')
ON CONFLICT (notification_type, channel, locale) DO NOTHING;

-- Reviews, trust, disputes, verifications
INSERT INTO notification_templates (id, name, notification_type, channel, locale, subject_template, body_template, variables_schema)
VALUES
    (gen_random_uuid(), 'review_requested_in_app', 'REVIEW_REQUESTED', 'IN_APP', 'en', '', 'Please submit your review for "{{deal_name}}".', '{"deal_name": "string", "deal_id": "uuid"}'),
    (gen_random_uuid(), 'review_requested_email', 'REVIEW_REQUESTED', 'EMAIL', 'en', 'Review requested for "{{deal_name}}"', 'Hi {{recipient_name}},

Please submit your review for "{{deal_name}}".

View deal: {{action_url}}', '{"deal_name": "string", "deal_id": "uuid", "recipient_name": "string"}'),

    (gen_random_uuid(), 'review_received_in_app', 'REVIEW_RECEIVED', 'IN_APP', 'en', '', 'You received a new review on "{{deal_name}}".', '{"deal_name": "string", "deal_id": "uuid"}'),
    (gen_random_uuid(), 'review_received_email', 'REVIEW_RECEIVED', 'EMAIL', 'en', 'New review on "{{deal_name}}"', 'Hi {{recipient_name}},

You received a new review on "{{deal_name}}".

View deal: {{action_url}}', '{"deal_name": "string", "deal_id": "uuid", "recipient_name": "string"}'),

    (gen_random_uuid(), 'trust_score_updated_in_app', 'TRUST_SCORE_UPDATED', 'IN_APP', 'en', '', 'Your trust score has been updated.', '{}'),
    (gen_random_uuid(), 'trust_score_updated_email', 'TRUST_SCORE_UPDATED', 'EMAIL', 'en', 'Trust score updated', 'Hi {{recipient_name}},

Your trust score has been updated.', '{"recipient_name": "string"}'),

    (gen_random_uuid(), 'dispute_opened_in_app', 'DISPUTE_OPENED', 'IN_APP', 'en', '', 'A dispute has been opened for "{{deal_name}}".', '{"deal_name": "string", "deal_id": "uuid"}'),
    (gen_random_uuid(), 'dispute_opened_email', 'DISPUTE_OPENED', 'EMAIL', 'en', 'Dispute opened for "{{deal_name}}"', 'Hi {{recipient_name}},

A dispute has been opened for "{{deal_name}}".

View deal: {{action_url}}', '{"deal_name": "string", "deal_id": "uuid", "recipient_name": "string"}'),

    (gen_random_uuid(), 'dispute_resolved_in_app', 'DISPUTE_RESOLVED', 'IN_APP', 'en', '', 'The dispute on "{{deal_name}}" has been resolved.', '{"deal_name": "string", "deal_id": "uuid"}'),
    (gen_random_uuid(), 'dispute_resolved_email', 'DISPUTE_RESOLVED', 'EMAIL', 'en', 'Dispute resolved for "{{deal_name}}"', 'Hi {{recipient_name}},

The dispute on "{{deal_name}}" has been resolved.

View deal: {{action_url}}', '{"deal_name": "string", "deal_id": "uuid", "recipient_name": "string"}'),

    (gen_random_uuid(), 'verification_approved_in_app', 'VERIFICATION_APPROVED', 'IN_APP', 'en', '', 'Your verification request has been approved.', '{}'),
    (gen_random_uuid(), 'verification_approved_email', 'VERIFICATION_APPROVED', 'EMAIL', 'en', 'Verification approved', 'Hi {{recipient_name}},

Your verification request has been approved.', '{"recipient_name": "string"}'),

    (gen_random_uuid(), 'verification_rejected_in_app', 'VERIFICATION_REJECTED', 'IN_APP', 'en', '', 'Your verification request has been rejected.', '{}'),
    (gen_random_uuid(), 'verification_rejected_email', 'VERIFICATION_REJECTED', 'EMAIL', 'en', 'Verification rejected', 'Hi {{recipient_name}},

Your verification request has been rejected.', '{"recipient_name": "string"}')
ON CONFLICT (notification_type, channel, locale) DO NOTHING;

-- Messaging
INSERT INTO notification_templates (id, name, notification_type, channel, locale, subject_template, body_template, variables_schema)
VALUES
    (gen_random_uuid(), 'message_received_in_app', 'MESSAGE_RECEIVED', 'IN_APP', 'en', '', 'You have a new message from {{sender_name}}.', '{"sender_name": "string"}'),
    (gen_random_uuid(), 'message_received_email', 'MESSAGE_RECEIVED', 'EMAIL', 'en', 'New message from {{sender_name}}', 'Hi {{recipient_name}},

You have a new message from {{sender_name}}.', '{"sender_name": "string", "recipient_name": "string"}'),

    (gen_random_uuid(), 'mentioned_in_app', 'MENTIONED', 'IN_APP', 'en', '', 'You were mentioned by {{sender_name}}.', '{"sender_name": "string"}'),
    (gen_random_uuid(), 'mentioned_email', 'MENTIONED', 'EMAIL', 'en', 'You were mentioned by {{sender_name}}', 'Hi {{recipient_name}},

You were mentioned by {{sender_name}}.', '{"sender_name": "string", "recipient_name": "string"}')
ON CONFLICT (notification_type, channel, locale) DO NOTHING;
