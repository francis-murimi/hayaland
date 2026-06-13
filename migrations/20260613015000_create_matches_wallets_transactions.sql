-- Matching, wallets, transactions, and transaction approvals.
CREATE TABLE IF NOT EXISTS match_suggestions (
    id UUID PRIMARY KEY,
    supplier_party_id UUID NOT NULL REFERENCES parties(id),
    consumer_party_id UUID NOT NULL REFERENCES parties(id),
    enhancer_party_id UUID NOT NULL REFERENCES parties(id),
    match_status TEXT NOT NULL DEFAULT 'PENDING' CHECK (match_status IN ('PENDING','ACCEPTED','DECLINED','EXPIRED','CONVERTED_TO_DEAL')),
    match_score DECIMAL NOT NULL,
    match_reason TEXT,
    resource_category_id UUID REFERENCES categories(id),
    need_category_id UUID REFERENCES categories(id),
    enhancement_category_id UUID REFERENCES categories(id),
    suggested_deal_value DECIMAL, -- in platform points
    generated_by TEXT NOT NULL DEFAULT 'ALGORITHM',
    expires_at TIMESTAMPTZ,
    converted_deal_id UUID REFERENCES deals(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS platform_wallets (
    id UUID PRIMARY KEY,
    party_id UUID NOT NULL UNIQUE REFERENCES parties(id) ON DELETE CASCADE,
    balance DECIMAL NOT NULL DEFAULT 0, -- platform points
    escrow_balance DECIMAL NOT NULL DEFAULT 0, -- points held in escrow
    pending_balance DECIMAL NOT NULL DEFAULT 0, -- points awaiting approval
    total_deposited DECIMAL NOT NULL DEFAULT 0,
    total_withdrawn DECIMAL NOT NULL DEFAULT 0,
    currency TEXT NOT NULL DEFAULT 'POINTS',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS transactions (
    id UUID PRIMARY KEY,
    deal_id UUID REFERENCES deals(id),
    agreement_id UUID REFERENCES agreements(id),
    milestone_id UUID REFERENCES milestones(id),
    transaction_type TEXT NOT NULL, -- DEPOSIT, WITHDRAWAL, ESCROW_HOLD, ESCROW_RELEASE, FEE, ADJUSTMENT
    from_party_id UUID REFERENCES parties(id),
    to_party_id UUID REFERENCES parties(id),
    amount DECIMAL NOT NULL, -- platform points
    currency TEXT NOT NULL DEFAULT 'POINTS',
    description TEXT,
    status TEXT NOT NULL DEFAULT 'PENDING', -- PENDING, VERIFIED, COMPLETE, REJECTED
    payment_method TEXT, -- e.g., BANK_TRANSFER, CASH, IN_KIND, OTHER (mirrors external settlement)
    external_reference TEXT, -- reference to physical transaction outside platform
    requires_approval BOOLEAN NOT NULL DEFAULT true,
    approvals_required INTEGER NOT NULL DEFAULT 2,
    approvals_received INTEGER NOT NULL DEFAULT 0,
    executed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS transaction_approvals (
    id UUID PRIMARY KEY,
    transaction_id UUID NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
    party_id UUID NOT NULL REFERENCES parties(id),
    approved_by_user_id UUID NOT NULL REFERENCES users(id),
    decision TEXT NOT NULL CHECK (decision IN ('APPROVED','REJECTED')),
    comment TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (transaction_id, party_id)
);

CREATE INDEX IF NOT EXISTS idx_match_suggestions_supplier ON match_suggestions(supplier_party_id);
CREATE INDEX IF NOT EXISTS idx_match_suggestions_consumer ON match_suggestions(consumer_party_id);
CREATE INDEX IF NOT EXISTS idx_match_suggestions_enhancer ON match_suggestions(enhancer_party_id);
CREATE INDEX IF NOT EXISTS idx_transactions_deal ON transactions(deal_id);
CREATE INDEX IF NOT EXISTS idx_transactions_status ON transactions(status);
CREATE INDEX IF NOT EXISTS idx_transaction_approvals_txn ON transaction_approvals(transaction_id);
CREATE INDEX IF NOT EXISTS idx_transaction_approvals_party ON transaction_approvals(party_id);
