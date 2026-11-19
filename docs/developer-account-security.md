# Developer Console account security

Open **Account security** in the Developer Console. Developers can review sign-in history, end sessions, trust or forget a browser, add an authenticator or passkey, and manage sign-in notifications from the same page. The control plane stores these settings independently of application users.

| Control | Behavior |
| --- | --- |
| Verify your identity | Use your password plus an enrolled authenticator or backup code, or verify with a registered passkey. Proof lasts five minutes on the current session. |
| Authenticator and backup codes | Sign-in requires a six-digit authenticator code or a single-use backup code once 2FA is enabled. Enrollment and removal end other developer sessions. Replacing codes invalidates the previous set. |
| Passkeys | Passkeys verify your identity for security changes. The console uses WebAuthn and stores registration and verification challenges server-side for five minutes, scoped to the current developer session. |
| Trusted browsers | A browser stays trusted for 30 days. Trust identifies the browser in activity and never skips 2FA. Forgetting a browser ends its other sessions. |
| Password recovery | A time-limited email link resets the password, revokes all developer sessions, and removes browser trust. Enrolled authenticators remain active. |
| Sign-in protection | Signet limits repeated account failures and detects distinct failed account sign-ins by IPv4 address or IPv6 prefix. Login history shows suspicious activity and the observed signals. Multiple independent signals flag possible account takeover for review. |

## Configuration

- Apply migration `020_developer_authenticators.sql` with the Developer Console API. Existing developer sessions remain valid.
- Put the exact Developer Console origin first in `CORS_ORIGINS`. Use HTTPS in production or `http://localhost:3000` for local development. Changing the WebAuthn host requires enrolling new passkeys for that host. [WebAuthn origin and relying party requirements](https://www.w3.org/TR/webauthn-3/)
- Configure `SMTP_HOST`, `SMTP_PORT`, and the sender for recovery and sign-in email. Supply `SMTP_USER` and `SMTP_PASS` if the relay requires credentials. Signet uses TLS on port 465 and STARTTLS on other ports. Delivery uses the runtime CA trust store. Suspicious events remain available in Login history when email delivery is unavailable.
- Configure `TRUSTED_PROXY_CIDRS` for the actual proxy hops, so sign-in reports use the network address the edge observed.
- Import an approved IP intelligence dataset into `account_ip_intelligence`. Supply normalized CIDR ranges and a future `expires_at`, with `latitude` and `longitude` for travel checks or `reputation` set to `elevated` or `high` for network risk. Refresh the data before it expires. Signet uses the most specific unexpired matching range for each signal. The console reports whether each type of data is available. These detections use IP estimates and can be affected by VPNs, proxies, and shared networks.

The PostgreSQL import accepts a CSV header `network,country,latitude,longitude,reputation,expires_at`. To upsert an approved file, stage and copy it within one `psql` session.

```sql
BEGIN;
CREATE TEMP TABLE ip_intelligence_import (LIKE account_ip_intelligence INCLUDING ALL) ON COMMIT DROP;
\copy ip_intelligence_import FROM 'ip-intelligence.csv' WITH (FORMAT csv, HEADER true)
INSERT INTO account_ip_intelligence (network, country, latitude, longitude, reputation, expires_at)
SELECT network, country, latitude, longitude, reputation, expires_at
FROM ip_intelligence_import WHERE expires_at > NOW()
ON CONFLICT (network) DO UPDATE SET
    country = EXCLUDED.country,
    latitude = EXCLUDED.latitude,
    longitude = EXCLUDED.longitude,
    reputation = EXCLUDED.reputation,
    expires_at = EXCLUDED.expires_at;
COMMIT;
```

## Verification

With disposable PostgreSQL and Redis services configured through `TEST_DATABASE_URL` and `TEST_REDIS_URL`, run the control-plane integration tests.

```sh
cargo test --manifest-path apps/auth-api/Cargo.toml --locked \
  --test phase07_control_plane_tests -- --include-ignored --test-threads=1
```
