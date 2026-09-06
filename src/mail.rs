use std::sync::{Arc, Mutex};

use serde_json::json;

use crate::error::Result;

/// Outbound mailer. Core never speaks SMTP.
#[derive(Clone)]
pub struct Mailer {
    inner: MailerKind,
    pub last_code: Arc<Mutex<Option<(String, String)>>>,
}

#[derive(Clone)]
enum MailerKind {
    Log,
    Http {
        endpoint: String,
        token: String,
    },
    Cloudflare {
        account_id: String,
        token: String,
        from: String,
    },
}

impl Mailer {
    pub fn from_env() -> Self {
        let last_code = Arc::new(Mutex::new(None));
        let cf_token = std::env::var("CF_EMAIL_TOKEN").unwrap_or_default();
        let cf_account_id = std::env::var("CF_ACCOUNT_ID").unwrap_or_default();
        if !cf_token.is_empty() && !cf_account_id.is_empty() {
            let from = std::env::var("CF_EMAIL_FROM")
                .unwrap_or_else(|_| "noreply@cloudcell.dev".to_string());
            return Self {
                inner: MailerKind::Cloudflare {
                    account_id: cf_account_id,
                    token: cf_token,
                    from,
                },
                last_code,
            };
        }
        let endpoint = std::env::var("MAIL_ENDPOINT").unwrap_or_default();
        let token = std::env::var("MAIL_TOKEN").unwrap_or_default();
        if !endpoint.is_empty() && !token.is_empty() {
            return Self {
                inner: MailerKind::Http { endpoint, token },
                last_code,
            };
        }
        Self {
            inner: MailerKind::Log,
            last_code,
        }
    }

    pub fn log() -> Self {
        Self {
            inner: MailerKind::Log,
            last_code: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn send_verification_code(&self, to: &str, code: &str) -> Result<()> {
        if let Ok(mut g) = self.last_code.lock() {
            *g = Some((to.to_string(), code.to_string()));
        }
        let subject = format!("Your verification code is {code}");
        let text = format!("Your verification code is: {code}\n\nThis code expires in 10 minutes.");
        let html = format!(
            "<div style=\"font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 480px; margin: 0 auto; padding: 24px; border: 1px solid #e5e7eb; border-radius: 8px;\">\
                <h2 style=\"color: #111827; margin-bottom: 16px;\">Cloudcell</h2>\
                <p style=\"color: #4b5563; font-size: 15px;\">Use this code to finish registration:</p>\
                <div style=\"font-size: 32px; font-weight: bold; letter-spacing: 6px; color: #2563eb; background: #f3f4f6; padding: 16px; text-align: center; border-radius: 6px; margin: 20px 0;\">{code}</div>\
                <p style=\"color: #6b7280; font-size: 13px;\">Expires in 10 minutes.</p>\
            </div>"
        );

        match &self.inner {
            MailerKind::Log => {
                tracing::info!(to, code, "verification code (mail not configured)");
                Ok(())
            }
            MailerKind::Cloudflare {
                account_id,
                token,
                from,
            } => {
                let url = format!(
                    "https://api.cloudflare.com/client/v4/accounts/{account_id}/email/sending/send"
                );
                let res = reqwest::Client::new()
                    .post(&url)
                    .bearer_auth(token)
                    .json(&json!({
                        "to": to,
                        "from": from,
                        "subject": subject,
                        "text": text,
                        "html": html,
                    }))
                    .send()
                    .await
                    .map_err(|e| anyhow::anyhow!("cloudflare email request failed: {e}"))?;
                if !res.status().is_success() {
                    let status = res.status();
                    let body = res.text().await.unwrap_or_default();
                    return Err(anyhow::anyhow!("cloudflare email error {status}: {body}").into());
                }
                Ok(())
            }
            MailerKind::Http { endpoint, token } => {
                let res = reqwest::Client::new()
                    .post(endpoint)
                    .bearer_auth(token)
                    .json(&json!({
                        "to": to,
                        "subject": subject,
                        "text": text,
                        "html": html,
                    }))
                    .send()
                    .await
                    .map_err(|e| anyhow::anyhow!("mail request failed: {e}"))?;
                if !res.status().is_success() {
                    let status = res.status();
                    let body = res.text().await.unwrap_or_default();
                    return Err(anyhow::anyhow!("mail provider {status}: {body}").into());
                }
                Ok(())
            }
        }
    }
}
