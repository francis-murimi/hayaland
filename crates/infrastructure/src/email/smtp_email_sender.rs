use application::email::EmailSender;
use application::errors::ApplicationError;
use async_trait::async_trait;
use lettre::message::Message;
use lettre::transport::smtp::authentication::Credentials;
use lettre::AsyncSmtpTransport;
use lettre::AsyncTransport;
use secrecy::ExposeSecret;
use tracing::{debug, error};

use crate::config::EmailSettings;

#[derive(Clone)]
pub struct SmtpEmailSender {
    transport: AsyncSmtpTransport<lettre::Tokio1Executor>,
    from: String,
    from_name: String,
}

impl SmtpEmailSender {
    pub fn new(settings: &EmailSettings) -> Result<Self, ApplicationError> {
        let creds = Credentials::new(
            settings.smtp_username.clone(),
            settings.smtp_password.expose_secret().to_string(),
        );

        let transport =
            AsyncSmtpTransport::<lettre::Tokio1Executor>::starttls_relay(&settings.smtp_host)
                .map_err(|e| {
                    error!(error = %e, "failed to build SMTP transport");
                    ApplicationError::Infrastructure(format!("invalid SMTP configuration: {e}"))
                })?
                .port(settings.smtp_port)
                .credentials(creds)
                .build();

        Ok(Self {
            transport,
            from: settings.from_address.clone(),
            from_name: settings.from_name.clone(),
        })
    }
}

#[async_trait]
impl EmailSender for SmtpEmailSender {
    async fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), ApplicationError> {
        let from = if self.from_name.is_empty() {
            self.from.clone()
        } else {
            format!("{} <{}>", self.from_name, self.from)
        };

        let message = Message::builder()
            .from(from.parse().map_err(|e| {
                ApplicationError::Infrastructure(format!("invalid from address: {e}"))
            })?)
            .to(to.parse().map_err(|e| {
                ApplicationError::Infrastructure(format!("invalid to address: {e}"))
            })?)
            .subject(subject)
            .body(body.to_string())
            .map_err(|e| ApplicationError::Infrastructure(format!("failed to build email: {e}")))?;

        debug!(to, subject, "sending email via SMTP");
        self.transport.send(message).await.map_err(|e| {
            error!(error = %e, to, "failed to send email");
            ApplicationError::EmailSendFailed
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EmailSettings;
    use secrecy::SecretString;

    fn settings(host: &str, port: u16, from: &str, from_name: &str) -> EmailSettings {
        EmailSettings {
            smtp_host: host.to_string(),
            smtp_port: port,
            smtp_username: "user".to_string(),
            smtp_password: SecretString::from("pass"),
            from_address: from.to_string(),
            from_name: from_name.to_string(),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn send_rejects_invalid_from_address() {
        let settings = settings("localhost", 587, "not-an-email", "");
        let sender = SmtpEmailSender::new(&settings).unwrap();
        let result = sender.send("to@example.com", "subject", "body").await;
        assert!(matches!(result, Err(ApplicationError::Infrastructure(_))));
    }

    #[tokio::test]
    async fn send_rejects_invalid_to_address() {
        let settings = settings("localhost", 587, "test@example.com", "");
        let sender = SmtpEmailSender::new(&settings).unwrap();
        let result = sender.send("not-an-email", "subject", "body").await;
        assert!(matches!(result, Err(ApplicationError::Infrastructure(_))));
    }

    #[tokio::test]
    async fn send_reports_failure_when_server_is_unreachable() {
        let settings = settings("127.0.0.1", 65534, "test@example.com", "Hayaland");
        let sender = SmtpEmailSender::new(&settings).unwrap();
        let result = sender.send("recipient@example.com", "subject", "body").await;
        assert!(matches!(result, Err(ApplicationError::EmailSendFailed)));
    }
}
