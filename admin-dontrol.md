# Admin Control Access Design

## Purpose

This document describes how administrators gain full platform-management access to the actions that are currently blocked by party/deal membership checks. The design keeps the existing REST endpoints and use-case structure; it only adds an `is_admin` bypass branch to the relevant authorization checks.

## Scope model

The production role definitions already grant the `admin` role everything needed:

```text
users:read, users:write, users:admin, users:delete,
admin:parties, admin:parties:read, admin:parties:write, admin:parties:delete,
admin:users, admin:deals, admin:transactions, admin:milestones, admin:*
```

A user with `admin:*` can act as an administrator in every domain. Domain-scoped admins (e.g. only `admin:deals`) are restricted to their domain.

No new database migration is required.

## Design principles

1. **Reuse the existing `is_admin` pattern.** Wallet reads, milestone reads/mutations, transaction approvals, and agreement reads already accept an `is_admin: bool` and skip membership checks when it is true.
2. **Keep `X-Party-ID` required.** The handler still resolves an `actor_party_id` from the header. Admins bypass the membership/participation check, but the party ID is retained for audit logging and history records.
3. **Scope-gate mutations.** State-changing handlers use `require_scope_or_admin(&ctx, FEATURE_SCOPE, ADMIN_SCOPE)` so regular users need the feature scope and admins need the domain admin scope (or `admin:*`).
4. **Preserve member/owner semantics for non-admins.** Only the admin branch is relaxed.

## Current limitations and how they are resolved

### 1. Wallet mutations

**Blocked today:**

- `POST /api/v1/parties/{id}/wallet/deposits`
- `POST /api/v1/parties/{id}/wallet/withdrawals`

These enforce that the caller is a member of the party and that the party participates in the deal. There is no admin flag.

**Resolution:**

- Add `is_admin: bool` to `DepositPointsCommand` and `WithdrawPointsCommand`.
- In `DepositPoints` and `WithdrawPoints`, skip the membership/participation checks when `is_admin` is true.
- In the handlers, require `payments:write` or `admin:transactions` and pass `is_admin` from the JWT scopes.

**Result:** admins can record deposits/withdrawals for any deal party. Regular users still need `payments:write` and membership.

### 2. State-changing deal actions

**Blocked today:** the handler already requires `deals:write` or `admin:deals`, but the use case still enforces membership of the acting party.

Affected endpoints:

- `POST   /api/v1/deals`
- `PUT/PATCH /api/v1/deals/{id}`
- `POST   /api/v1/deals/{id}/submit`
- `POST   /api/v1/deals/{id}/transitions`
- `POST   /api/v1/deals/{id}/terms`
- `POST   /api/v1/deals/{id}/terms/{term_id}/counter`
- `POST   /api/v1/deals/{id}/terms/{term_id}/accept`
- `POST   /api/v1/deals/{id}/terms/{term_id}/reject`
- `POST   /api/v1/deals/{id}/terms/{term_id}/withdraw`
- `POST   /api/v1/deals/{id}/value-distribution`
- `POST   /api/v1/deals/{id}/agreement/sign`

**Resolution:**

- Add `is_admin: bool` to every state-changing deal command (`CreateDealCommand`, `UpdateDealCommand`, `SubmitDealCommand`, `ExecuteTransitionCommand`, `ProposeTermCommand`, `CounterTermCommand`, `TermActionCommand`, `SetValueDistributionCommand`, and `SignAgreementCommand`).
- In each corresponding use case, wrap the membership/participation checks in `if !is_admin { ... }`.
- In the API handlers, compute `is_admin = ctx.has_scope("admin:deals") || ctx.has_scope("admin:*")` and pass it into the command.
- Keep `X-Party-ID` resolution unchanged.

Special cases:

- **Create deal:** still verify the actor party exists and is active and determine its deal role, but skip the `is_user_member_of_party` check for admins.
- **Update deal:** skip the "only initiator party members" check for admins.
- **Submit / transition / terms / value distribution / sign agreement:** skip membership/participation checks for admins.

**Result:** admins can perform any deal state change on behalf of any participating party by supplying that party's ID in `X-Party-ID`. Regular users remain bound to their own memberships.

### 3. Party deletion with active deals

**Blocked today:** `DELETE /api/v1/parties/{id}` returns `PartyHasActiveDeals` even for admins.

**Resolution:**

The `SoftDeleteParty` use case already receives `is_admin: bool`. Modify it to skip the active-deals check when `is_admin` is true:

```rust
if !is_admin {
    let active_deals = self.repo.count_active_deals(party_id).await?;
    if active_deals > 0 {
        return Err(ApplicationError::PartyHasActiveDeals);
    }
}
```

The handler already passes `is_admin(&ctx)` and requires `parties:write` or `admin:parties`, so no handler change is needed.

**Trade-off:** this allows a platform admin to deactivate a party that still has open deals. The deal records remain in the database; only the party's `is_active` flag changes. This is intentional platform-moderation behavior.

**Result:** admins can soft-delete any party, including parties with active deals. Owners still cannot delete parties with active deals.

## New and changed use cases

No entirely new use-case files are required. The following existing use cases are extended with an `is_admin` branch:

| Use case | Change |
|----------|--------|
| `DepositPoints` | Skip membership/participation checks when admin. |
| `WithdrawPoints` | Skip membership/participation checks when admin. |
| `CreateDeal` | Skip `is_user_member_of_party` for admins. |
| `UpdateDeal` | Skip initiator-membership check for admins. |
| `SubmitDeal` | Skip participation check for admins. |
| `ExecuteTransition` | Skip participation check for admins. |
| `ProposeTerm` | Skip participation check for admins. |
| `CounterTerm` | Skip participation check for admins. |
| `AcceptTerm` | Skip participation check for admins. |
| `RejectTerm` | Skip participation check for admins. |
| `WithdrawTerm` | Skip participation check for admins. |
| `SetValueDistribution` | Skip membership/participation checks for admins. |
| `SignAgreement` | Skip membership check for admins. |
| `SoftDeleteParty` | Skip active-deals check for admins. |

## Handler changes

- `deposit_points.rs` and `withdraw_points.rs`: add `require_scope_or_admin("payments:write", "admin:transactions")` and pass `is_transaction_admin(&ctx)`.
- `create_deal.rs`, `update_deal.rs`, `submit_deal.rs`, `execute_transition.rs`, `terms.rs`, `value_distribution.rs`, `sign_agreement.rs`: compute `is_admin` from `admin:deals`/`admin:*` and pass it to the command.

No new routes are required.

## Tests to add

- `admin_can_deposit_points_for_party`
- `admin_can_withdraw_points_for_party`
- `admin_can_create_deal_without_membership`
- `admin_can_update_deal_without_membership`
- `admin_can_submit_and_transition_deal_without_membership`
- `admin_can_propose_and_sign_terms_without_membership`
- `admin_can_set_value_distribution_without_membership`
- `admin_can_sign_agreement_without_membership`
- `admin_can_delete_party_with_active_deals`

Existing unit tests must be updated to pass `is_admin: false` in the affected commands.

## Acceptance criteria

- `cargo fmt --check` passes.
- `cargo clippy -- -D warnings` passes.
- `cargo test --workspace` passes.
- `cargo sqlx prepare --workspace` produces no diff.
- `cargo llvm-cov --workspace` stays above 80% line coverage.
- An admin token can execute every listed endpoint on behalf of a non-member party, while a non-admin token still requires membership.
