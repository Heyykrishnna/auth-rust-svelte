use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserSecuritySettings {
    pub user_id: Uuid,
    pub mfa_enabled: bool,
    pub two_factor_method: String,
    pub recovery_email: Option<String>,
    pub recovery_phone: Option<String>,
    pub security_notifications_enabled: bool,
    pub notify_new_login: bool,
    pub notify_suspicious_activity: bool,
    pub notify_security_changes: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSecuritySettingsRequest {
    pub recovery_email: Option<String>,
    pub recovery_phone: Option<String>,
    pub security_notifications_enabled: Option<bool>,
    pub notify_new_login: Option<bool>,
    pub notify_suspicious_activity: Option<bool>,
    pub notify_security_changes: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserTotpCredential {
    pub id: Uuid,
    pub user_id: Uuid,
    pub secret_base32: String,
    pub verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotpEnrollResponse {
    pub secret: String,
    pub qr_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyTotpRequest {
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserPasskey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub credential_id: String,
    pub public_key_spki: String,
    pub name: String,
    pub aaguid: Option<String>,
    pub sign_count: i64,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasskeyInfo {
    pub id: Uuid,
    pub credential_id: String,
    pub name: String,
    pub aaguid: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

impl From<UserPasskey> for PasskeyInfo {
    fn from(p: UserPasskey) -> Self {
        Self {
            id: p.id,
            credential_id: p.credential_id,
            name: p.name,
            aaguid: p.aaguid,
            created_at: p.created_at,
            last_used_at: p.last_used_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasskeyRegistrationOptions {
    pub challenge: String,
    pub rp_name: String,
    pub rp_id: String,
    pub user_id: String,
    pub user_email: String,
    pub user_display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyPasskeyRegistrationRequest {
    pub credential_id: String,
    pub public_key_spki: String,
    pub name: String,
    pub client_data_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserBackupCode {
    pub id: Uuid,
    pub user_id: Uuid,
    pub code_hash: String,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupCodesResponse {
    pub codes: Vec<String>,
    pub remaining_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyBackupCodeRequest {
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepUpChallengeRequest {
    pub password: Option<String>,
    pub totp_code: Option<String>,
    pub backup_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepUpTokenResponse {
    pub step_up_token: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserDevice {
    pub id: Uuid,
    pub user_id: Uuid,
    pub fingerprint: String,
    pub device_name: String,
    pub device_type: String,
    pub trusted: bool,
    pub trusted_until: Option<DateTime<Utc>>,
    pub last_ip: Option<String>,
    pub last_location: Option<String>,
    pub last_active_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: Uuid,
    pub device_name: String,
    pub device_type: String,
    pub trusted: bool,
    pub trusted_until: Option<DateTime<Utc>>,
    pub last_ip: Option<String>,
    pub last_location: Option<String>,
    pub last_active_at: DateTime<Utc>,
    pub is_current: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserSecurityAlert {
    pub id: Uuid,
    pub user_id: Uuid,
    pub alert_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub ip_address: Option<String>,
    pub location: Option<String>,
    pub resolved: bool,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAlertInfo {
    pub id: Uuid,
    pub alert_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub ip_address: Option<String>,
    pub location: Option<String>,
    pub resolved: bool,
    pub created_at: DateTime<Utc>,
}

impl From<UserSecurityAlert> for SecurityAlertInfo {
    fn from(a: UserSecurityAlert) -> Self {
        Self {
            id: a.id,
            alert_type: a.alert_type,
            severity: a.severity,
            title: a.title,
            description: a.description,
            ip_address: a.ip_address,
            location: a.location,
            resolved: a.resolved,
            created_at: a.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginHistoryEntry {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub location: Option<String>,
    pub device_name: Option<String>,
    pub event: String,
    pub status: String,
    pub suspicious: bool,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BruteForceStatus {
    pub protected: bool,
    pub failed_attempts_past_24h: u32,
    pub consecutive_failures: u32,
    pub max_attempts_threshold: u32,
    pub is_locked: bool,
    pub lockout_expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialStuffingStatus {
    pub detection_active: bool,
    pub risk_level: String,
    pub stuffing_events_blocked_7d: u32,
    pub analyzed_subnets_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpossibleTravelStatus {
    pub detection_active: bool,
    pub anomalies_detected_30d: u32,
    pub velocity_threshold_kmh: f64,
    pub last_calculated_velocity_kmh: Option<f64>,
    pub last_hop_description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpReputationResult {
    pub ip: String,
    pub score: u32,
    pub risk_level: String,
    pub classification: String,
    pub is_datacenter: bool,
    pub is_vpn_proxy: bool,
    pub is_tor: bool,
    pub asn: Option<String>,
    pub country: Option<String>,
    pub city: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatIntelligenceReport {
    pub ato_risk_score: u32,
    pub ato_risk_level: String,
    pub brute_force_status: BruteForceStatus,
    pub credential_stuffing_status: CredentialStuffingStatus,
    pub impossible_travel_status: ImpossibleTravelStatus,
    pub ip_reputation: IpReputationResult,
    pub active_alerts_count: usize,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountRecoveryStatus {
    pub primary_email: String,
    pub recovery_email: Option<String>,
    pub recovery_phone: Option<String>,
    pub backup_codes_remaining: usize,
    pub mfa_enabled: bool,
    pub passkeys_count: usize,
    pub recovery_readiness_score: u32,
    pub recommendations: Vec<String>,
}
