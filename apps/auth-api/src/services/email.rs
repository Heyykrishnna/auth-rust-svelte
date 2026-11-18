use lettre::message::{header::ContentType, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use tracing::{error, info, warn};

use crate::config::AppConfig;
use crate::errors::AppError;

#[derive(Debug, Clone)]
pub struct EmailBrand {
    pub app_name: String,
    pub primary_color: String,
    pub support_email: Option<String>,
}

impl EmailBrand {
    pub fn from_environment_branding(branding: &serde_json::Value, fallback_name: &str) -> Self {
        let app_name = branding
            .get("app_name")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(fallback_name)
            .to_string();
        let primary_color = branding
            .get("primary_color")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| s.starts_with('#') && (s.len() == 4 || s.len() == 7))
            .unwrap_or("#8b5cf6")
            .to_string();
        let support_email = branding
            .get("support_email")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string);
        Self {
            app_name,
            primary_color,
            support_email,
        }
    }
}

pub async fn send_branded_otp_email(
    config: &AppConfig,
    brand: &EmailBrand,
    to_email: &str,
    to_name: &str,
    otp: &str,
) -> Result<(), AppError> {
    let host = match &config.smtp_host {
        Some(h) if !h.trim().is_empty() => h.trim(),
        _ => {
            warn!(
                to = to_email,
                brand = %brand.app_name,
                "SMTP host not configured; skipping branded OTP email dispatch"
            );
            return Ok(());
        }
    };

    let from_header = if config.smtp_from.contains('<') {
        config.smtp_from.clone()
    } else {
        format!("{} <{}>", brand.app_name, config.smtp_from)
    };
    let to_header = if to_name.trim().is_empty() {
        to_email.to_string()
    } else {
        format!("{} <{}>", to_name.trim(), to_email)
    };
    let support_line = brand
        .support_email
        .as_deref()
        .map(|s| format!("\n\nNeed help? Contact {s}."))
        .unwrap_or_default();
    let subject = format!("{} - Your Verification Code", brand.app_name);
    let text_body = format!(
        "Hello {},\n\nYour {} verification code is: {}\n\nThis code will expire in 10 minutes.{}",
        if to_name.trim().is_empty() {
            "there"
        } else {
            to_name.trim()
        },
        brand.app_name,
        otp,
        support_line
    );
    let html_body = format!(
        "<!DOCTYPE html><html><body style=\"font-family:sans-serif;background:#0a0a14;color:#f1f5f9;padding:32px;\"><div style=\"max-width:520px;margin:0 auto;background:#121226;border-radius:12px;padding:32px;text-align:center;\"><h2 style=\"color:{};margin-top:0;\">{}</h2><p>Use the verification code below:</p><div style=\"font-family:monospace;font-size:32px;font-weight:700;letter-spacing:6px;margin:20px 0;\">{}</div><p style=\"color:#94a3b8;font-size:13px;\">This code expires in 10 minutes.</p></div></body></html>",
        brand.primary_color, brand.app_name, otp
    );

    send_smtp_message(
        config,
        host,
        from_header,
        to_header,
        to_email,
        subject,
        text_body,
        html_body,
    )
    .await
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub async fn send_organization_invitation_email(
    config: &AppConfig,
    brand: &EmailBrand,
    to_email: &str,
    org_name: &str,
    role_slug: &str,
    invitation_token: &str,
) -> Result<(), AppError> {
    let host = match &config.smtp_host {
        Some(h) if !h.trim().is_empty() => h.trim(),
        _ => {
            warn!(
                to = to_email,
                org = org_name,
                "SMTP host not configured; skipping organization invitation email dispatch"
            );
            return Ok(());
        }
    };

    let from_header = if config.smtp_from.contains('<') {
        config.smtp_from.clone()
    } else {
        format!("{} <{}>", brand.app_name, config.smtp_from)
    };
    let subject = format!("You have been invited to join {}", org_name);
    let accept_url = format!(
        "{}/portal/login?invitation_token={}",
        config.issuer_url.trim_end_matches('/'),
        invitation_token
    );
    let safe_org = escape_html(org_name);
    let safe_role = escape_html(role_slug);
    let safe_brand = escape_html(&brand.app_name);
    let safe_token = escape_html(invitation_token);
    let safe_url = escape_html(&accept_url);

    let text_body = format!(
        "You have been invited to join {} on {} with the role {}.\n\nAccept your invitation: {}\n\nInvitation token: {}\n\nThis invitation is single-use and expires automatically.",
        org_name, brand.app_name, role_slug, accept_url, invitation_token
    );
    let html_body = format!(
        "<!DOCTYPE html><html><body style=\"font-family:-apple-system,BlinkMacSystemFont,sans-serif;background:#0b0c0e;color:#f3f4f6;padding:32px;\"><div style=\"max-width:520px;margin:0 auto;background:#15171b;border:1px solid #23262e;border-radius:8px;padding:32px;\"><p style=\"color:#9ca3af;font-size:12px;text-transform:uppercase;letter-spacing:0.08em;margin:0 0 12px;\">{safe_brand}</p><h2 style=\"color:#ffffff;margin:0 0 12px;font-size:20px;\">Join {safe_org}</h2><p style=\"color:#d1d5db;font-size:14px;line-height:1.5;margin:0 0 20px;\">You have been invited to join <strong>{safe_org}</strong> with role <code>{safe_role}</code>.</p><a href=\"{safe_url}\" style=\"display:inline-block;background:#ffffff;color:#0b0c0e;text-decoration:none;font-weight:600;padding:10px 18px;border-radius:6px;font-size:14px;\">Accept Invitation</a><p style=\"color:#9ca3af;font-size:12px;margin:24px 0 6px;\">Or use your one-time invitation token:</p><div style=\"font-family:monospace;font-size:12px;background:#0b0c0e;border:1px solid #23262e;padding:10px 12px;border-radius:6px;word-break:break-all;color:#f3f4f6;\">{safe_token}</div></div></body></html>"
    );

    send_smtp_message(
        config,
        host,
        from_header,
        to_email.to_string(),
        to_email,
        subject,
        text_body,
        html_body,
    )
    .await
}

pub async fn send_branded_password_reset_email(
    config: &AppConfig,
    brand: &EmailBrand,
    to_email: &str,
    to_name: &str,
    reset_token: &str,
) -> Result<(), AppError> {
    let host = match &config.smtp_host {
        Some(h) if !h.trim().is_empty() => h.trim(),
        _ => {
            warn!(
                to = to_email,
                brand = %brand.app_name,
                "SMTP host not configured; skipping branded password reset email dispatch"
            );
            return Ok(());
        }
    };

    let from_header = if config.smtp_from.contains('<') {
        config.smtp_from.clone()
    } else {
        format!("{} <{}>", brand.app_name, config.smtp_from)
    };
    let to_header = if to_name.trim().is_empty() {
        to_email.to_string()
    } else {
        format!("{} <{}>", to_name.trim(), to_email)
    };
    let base_origin = config
        .cors_origins
        .first()
        .cloned()
        .unwrap_or_else(|| "http://localhost:3000".to_string());
    let reset_url = format!("{}/reset-password?token={}", base_origin, reset_token);
    let subject = format!("{} - Password Reset Request", brand.app_name);
    let text_body = format!(
        "Hello {},\n\nUse the link below to reset your {} password:\n\n{}\n\nThis link expires in 15 minutes.",
        if to_name.trim().is_empty() {
            "there"
        } else {
            to_name.trim()
        },
        brand.app_name,
        reset_url
    );
    let html_body = format!(
        "<!DOCTYPE html><html><body style=\"font-family:sans-serif;background:#0a0a14;color:#f1f5f9;padding:32px;\"><div style=\"max-width:520px;margin:0 auto;background:#121226;border-radius:12px;padding:32px;text-align:center;\"><h2 style=\"color:{};margin-top:0;\">{}</h2><p>Reset your password using the button below:</p><a href=\"{}\" style=\"display:inline-block;background:{};color:#fff;padding:12px 24px;border-radius:8px;text-decoration:none;font-weight:600;\">Reset Password</a><p style=\"color:#94a3b8;font-size:13px;margin-top:20px;\">This link expires in 15 minutes.</p></div></body></html>",
        brand.primary_color, brand.app_name, reset_url, brand.primary_color
    );

    send_smtp_message(
        config,
        host,
        from_header,
        to_header,
        to_email,
        subject,
        text_body,
        html_body,
    )
    .await
}

pub async fn send_registration_otp(
    config: &AppConfig,
    to_email: &str,
    to_name: &str,
    otp: &str,
) -> Result<(), AppError> {
    let host = match &config.smtp_host {
        Some(h) if !h.trim().is_empty() => h.trim(),
        _ => {
            warn!(
                to = to_email,
                "SMTP host not configured; skipping registration OTP email dispatch"
            );
            return Ok(());
        }
    };

    let from_header = if config.smtp_from.contains('<') {
        config.smtp_from.clone()
    } else {
        format!("{} <{}>", config.smtp_from_name, config.smtp_from)
    };

    let to_header = if to_name.trim().is_empty() {
        to_email.to_string()
    } else {
        format!("{} <{}>", to_name.trim(), to_email)
    };

    let subject = "Your Verification Code".to_string();

    let text_body = format!(
        "Hello {},\n\nYour verification code is: {}\n\nThis code will expire in 10 minutes.\n\nIf you did not request this code, you can safely ignore this email.",
        if to_name.trim().is_empty() { "there" } else { to_name.trim() },
        otp
    );

    let html_body = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Verification Code</title>
</head>
<body style="margin: 0; padding: 0; background-color: #0a0a14; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; color: #f1f5f9;">
  <table role="presentation" width="100%" cellspacing="0" cellpadding="0" style="background-color: #0a0a14; padding: 40px 20px;">
    <tr>
      <td align="center">
        <table role="presentation" width="100%" style="max-width: 520px; background-color: #121226; border: 1px solid rgba(139, 92, 246, 0.25); border-radius: 16px; overflow: hidden; box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);">
          <tr>
            <td style="padding: 32px 32px 24px 32px; text-align: center; background: linear-gradient(180deg, rgba(139, 92, 246, 0.12) 0%, transparent 100%);">
              <div style="display: inline-block; font-size: 24px; font-weight: 700; color: #a78bfa; letter-spacing: -0.5px;">
                SecureAuth
              </div>
            </td>
          </tr>
          <tr>
            <td style="padding: 0 32px 32px 32px; text-align: center;">
              <h1 style="margin: 0 0 12px 0; font-size: 22px; font-weight: 700; color: #ffffff; letter-spacing: -0.5px;">
                Verify your email address
              </h1>
              <p style="margin: 0 0 28px 0; font-size: 15px; line-height: 1.5; color: #94a3b8;">
                Use the verification code below to complete your registration.
              </p>
              <div style="background: rgba(139, 92, 246, 0.08); border: 1px solid rgba(139, 92, 246, 0.35); border-radius: 12px; padding: 20px 24px; margin: 0 auto 28px auto; display: inline-block;">
                <span style="font-family: monospace; font-size: 36px; font-weight: 700; letter-spacing: 8px; color: #a78bfa;">
                  {}
                </span>
              </div>
              <p style="margin: 0 0 8px 0; font-size: 13px; color: #64748b;">
                This code expires in <strong>10 minutes</strong>.
              </p>
              <p style="margin: 0; font-size: 13px; color: #64748b;">
                If you did not request this verification code, please disregard this email.
              </p>
            </td>
          </tr>
        </table>
      </td>
    </tr>
  </table>
</body>
</html>"#,
        otp
    );

    send_smtp_message(
        config,
        host,
        from_header,
        to_header,
        to_email,
        subject,
        text_body,
        html_body,
    )
    .await
}

pub async fn send_password_reset_email(
    config: &AppConfig,
    to_email: &str,
    to_name: &str,
    reset_token: &str,
) -> Result<(), AppError> {
    let host = match &config.smtp_host {
        Some(h) if !h.trim().is_empty() => h.trim(),
        _ => {
            warn!(
                to = to_email,
                "SMTP host not configured; skipping password reset email dispatch"
            );
            return Ok(());
        }
    };

    let from_header = if config.smtp_from.contains('<') {
        config.smtp_from.clone()
    } else {
        format!("{} <{}>", config.smtp_from_name, config.smtp_from)
    };

    let to_header = if to_name.trim().is_empty() {
        to_email.to_string()
    } else {
        format!("{} <{}>", to_name.trim(), to_email)
    };

    let base_origin = config
        .cors_origins
        .first()
        .cloned()
        .unwrap_or_else(|| "http://localhost:3000".to_string());
    let reset_url = format!("{}/reset-password?token={}", base_origin, reset_token);

    let subject = "Password Reset Request".to_string();

    let text_body = format!(
        "Hello {},\n\nWe received a request to reset your password. Use the link below to set a new password:\n\n{}\n\nThis link will expire in 15 minutes.\n\nIf you did not request a password reset, you can safely ignore this email.",
        if to_name.trim().is_empty() { "there" } else { to_name.trim() },
        reset_url
    );

    let html_body = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Reset Your Password</title>
</head>
<body style="margin: 0; padding: 0; background-color: #0a0a14; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; color: #f1f5f9;">
  <table role="presentation" width="100%" cellspacing="0" cellpadding="0" style="background-color: #0a0a14; padding: 40px 20px;">
    <tr>
      <td align="center">
        <table role="presentation" width="100%" style="max-width: 520px; background-color: #121226; border: 1px solid rgba(139, 92, 246, 0.25); border-radius: 16px; overflow: hidden; box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);">
          <tr>
            <td style="padding: 32px 32px 24px 32px; text-align: center; background: linear-gradient(180deg, rgba(139, 92, 246, 0.12) 0%, transparent 100%);">
              <div style="display: inline-block; font-size: 24px; font-weight: 700; color: #a78bfa; letter-spacing: -0.5px;">
                SecureAuth
              </div>
            </td>
          </tr>
          <tr>
            <td style="padding: 0 32px 32px 32px; text-align: center;">
              <h1 style="margin: 0 0 12px 0; font-size: 22px; font-weight: 700; color: #ffffff; letter-spacing: -0.5px;">
                Reset Your Password
              </h1>
              <p style="margin: 0 0 28px 0; font-size: 15px; line-height: 1.5; color: #94a3b8;">
                Click the button below to reset your password. This link is valid for 15 minutes.
              </p>
              <div style="margin: 0 auto 28px auto;">
                <a href="{}" style="display: inline-block; background-color: #8b5cf6; color: #ffffff; font-weight: 600; text-decoration: none; padding: 14px 28px; border-radius: 8px;">
                  Reset Password
                </a>
              </div>
              <p style="margin: 0; font-size: 13px; color: #64748b;">
                If you did not request this password reset, please ignore this email.
              </p>
            </td>
          </tr>
        </table>
      </td>
    </tr>
  </table>
</body>
</html>"#,
        reset_url
    );

    send_smtp_message(
        config,
        host,
        from_header,
        to_header,
        to_email,
        subject,
        text_body,
        html_body,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn send_smtp_message(
    config: &AppConfig,
    host: &str,
    from_header: String,
    to_header: String,
    to_email: &str,
    subject: String,
    text_body: String,
    html_body: String,
) -> Result<(), AppError> {
    static SMTP_PERMITS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(16);
    let _permit = SMTP_PERMITS.try_acquire().map_err(|_| AppError::TooManyRequests("Email delivery capacity reached; retry shortly".to_string()))?;
    let clean_user = config.smtp_user.as_deref().map(|u| u.trim().to_string());

    let clean_pass = config
        .smtp_pass
        .as_deref()
        .map(|p| p.replace(' ', "").trim().to_string());

    let mut message_builder = Message::builder()
        .from(
            from_header
                .parse()
                .map_err(|e| AppError::Internal(format!("Invalid SMTP From header: {e}")))?,
        )
        .to(to_header
            .parse()
            .map_err(|e| AppError::Internal(format!("Invalid recipient email address: {e}")))?)
        .subject(subject);

    if let Some(user_addr) = &clean_user {
        if let (Ok(env_from), Ok(env_to)) = (user_addr.parse(), to_email.parse()) {
            if let Ok(envelope) = lettre::address::Envelope::new(Some(env_from), vec![env_to]) {
                message_builder = message_builder.envelope(envelope);
            }
        }
    }

    let email = message_builder
        .multipart(
            MultiPart::alternative()
                .singlepart(SinglePart::plain(text_body))
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_HTML)
                        .body(html_body),
                ),
        )
        .map_err(|e| AppError::Internal(format!("Failed to construct email: {e}")))?;

    let mut transport_builder = if config.smtp_port == 465 {
        AsyncSmtpTransport::<Tokio1Executor>::relay(host)
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
    }
    .map_err(|e| AppError::Internal(format!("Failed to create SMTP transport: {e}")))?
    .port(config.smtp_port);

    if let (Some(user), Some(pass)) = (clean_user, clean_pass) {
        transport_builder = transport_builder.credentials(Credentials::new(user, pass));
    }

    let transport = transport_builder.build();

    let result = tokio::time::timeout(std::time::Duration::from_secs(10), transport.send(email)).await;
    match result {
        Ok(Ok(_)) => {
            metrics::counter!("auth_email_delivery_total", "result" => "success").increment(1);
            info!(to = to_email, "Email dispatched successfully");
            Ok(())
        }
        Ok(Err(err)) => {
            metrics::counter!("auth_email_delivery_total", "result" => "failure").increment(1);
            error!(to = to_email, error = %err, "Failed to send email via SMTP");
            Err(AppError::Internal(format!("Failed to send email: {err}")))
        }
        Err(_) => {
            metrics::counter!("auth_email_delivery_total", "result" => "timeout").increment(1);
            Err(AppError::Internal("Email delivery timed out".to_string()))
        }
    }
}
