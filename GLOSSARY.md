# Identity Infrastructure Platform

A multi-tenant authentication and central identity platform that lets independent applications delegate authentication, Single Sign-On, authorization, and identity synchronization to a shared identity core.

## Language

### Control Plane

**Developer**:
A human operator who administers applications and environments in the Control Plane.
_Avoid_: Admin, customer, end user

**Workspace**:
An administrative boundary that groups Developers and owns one or more Applications.
_Avoid_: Account, developer org, team tenant

**Application**:
A distinct software product (such as Ferrum or Elysiar) registered on the platform.
_Avoid_: Project, client, service

**Environment**:
An isolated runtime instance of an Application (`development` or `production`) that owns its own credentials, domains, memberships, sessions, organizations, roles, and webhooks.
_Avoid_: Stage, tenant, namespace, instance

**Publishable Key**:
A public, environment-scoped credential (`pk_test_*` or `pk_live_*`) that identifies an Environment to browser SDKs and OIDC clients without granting privileged access.
_Avoid_: Public secret, client key

**Secret Key**:
A privileged, environment-scoped credential (`sk_test_*` or `sk_live_*`) used exclusively by a consuming application's backend for server-to-server management and synchronization.
_Avoid_: API token, private key

### Identity & Access Plane

**User**:
A globally unique human identity shared across Applications, owning credentials, verified email addresses, and external OAuth links.
_Avoid_: Account, member, profile

**Application User**:
The membership linking a global User to a specific Environment, carrying environment-scoped status, metadata, and roles.
_Avoid_: Tenant user, app profile, account

**SSO Session**:
A central Identity Provider login session bound to a global User, enabling passwordless OIDC sign-in across Applications when policy permits.
_Avoid_: Global token, master cookie

**App Session**:
An authentication session bound strictly to one Application User inside a single Environment.
_Avoid_: Login session, user session

**Enrollment Policy**:
The rule on an Environment determining whether a global User authenticated via SSO joins automatically (`auto_join`), after explicit approval (`consent_required`), or only with an invitation (`invite_only`).
_Avoid_: Auto-signup, provisioning mode

**Organization**:
A B2B group or team owned by an Environment to which Application Users belong with scoped roles and permissions.
_Avoid_: Company, workspace, tenant

**Hosted Auth Portal**:
The central Identity Provider web interface where Users sign in, verify emails, approve app consent, and manage SSO sessions.
_Avoid_: Login widget, dashboard

**Developer Control Plane**:
The management surface and APIs where Developers configure Workspaces, Applications, Environments, credentials, domains, webhooks, and analytics.
_Avoid_: Admin panel, backend portal

### Synchronization

**Identity Version**:
A strictly increasing counter on a synced entity (User, Application User, Organization, Organization Member) used by consuming services to reject stale or out-of-order updates.
_Avoid_: Revision, etag, timestamp lock

**Outbox Event**:
A durable, versioned identity lifecycle record written atomically with a state change and fanned out to registered Webhook Endpoints.
_Avoid_: Notification, job, message

