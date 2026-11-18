---
status: accepted
---

# 0001: Application Environments as the Tenant Boundary

We model every `Application` as owning isolated `Application Environments` (`development` and `production`), and make `environment_id`—not `application_id`—the tenant foreign key for `application_users`, `app_sessions`, `application_domains`, `application_credentials`, `organizations`, `roles`, `webhook_endpoints`, `outbox_events`, and tenant-scoped `audit_logs`.

## Considered Options

- **Attach tenant tables directly to `applications`**: Simpler initial ER diagram, but forces development test users, `localhost` redirect URIs, test webhooks, and test organizations to collide with production data or rely on fragile environment-flag filters in every query.
- **Separate `application_environments` entity (`environment_id` as tenant boundary)**: Mirrors real-world auth platforms (e.g., Clerk dev/prod instances) and guarantees hard database-level isolation between test and live data using composite foreign keys and PostgreSQL RLS.

## Consequences

- Creating an application atomically provisions its `development` and `production` environments, default roles, and initial publishable key.
- Credential prefixes (`pk_test_*` / `sk_test_*` vs. `pk_live_*` / `sk_live_*`) resolve unambiguously to a single `environment_id`.
- Cross-environment isolation tests must cover both cross-application (`Ferrum prod` vs. `Elysiar prod`) and intra-application (`Ferrum dev` vs. `Ferrum prod`) boundaries.

