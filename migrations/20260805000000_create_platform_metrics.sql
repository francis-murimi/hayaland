-- Platform metrics: daily snapshots for analytics dashboards and reports.
CREATE TABLE IF NOT EXISTS platform_metrics (
    date DATE PRIMARY KEY,
    total_deals INTEGER NOT NULL DEFAULT 0,
    deals_completed INTEGER NOT NULL DEFAULT 0,
    deals_disputed INTEGER NOT NULL DEFAULT 0,
    deals_cancelled INTEGER NOT NULL DEFAULT 0,
    deals_by_status JSONB NOT NULL DEFAULT '{}',
    total_parties INTEGER NOT NULL DEFAULT 0,
    active_parties INTEGER NOT NULL DEFAULT 0,
    total_users INTEGER NOT NULL DEFAULT 0,
    active_users INTEGER NOT NULL DEFAULT 0,
    avg_deal_value DECIMAL(19, 4) NOT NULL DEFAULT 0,
    total_escrow_held DECIMAL(19, 4) NOT NULL DEFAULT 0,
    total_fees_collected DECIMAL(19, 4) NOT NULL DEFAULT 0,
    total_reviews INTEGER NOT NULL DEFAULT 0,
    avg_review_score DECIMAL(3, 2) NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_platform_metrics_date ON platform_metrics(date DESC);
