pub mod in_memory_email_queue;
pub mod smtp_email_sender;

pub use in_memory_email_queue::{run_worker, InMemoryEmailQueue};
pub use smtp_email_sender::SmtpEmailSender;
