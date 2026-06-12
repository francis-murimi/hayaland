use application::email::queue::{EmailQueue, EmailQueueItem};
use application::email::EmailSender;
use application::errors::ApplicationError;
use async_trait::async_trait;
use infrastructure::email::{run_worker, InMemoryEmailQueue};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct FakeSender {
    sent: Mutex<Vec<(String, String, String)>>,
    failures_before_success: Mutex<u32>,
    permanent_failure: Mutex<bool>,
}

impl FakeSender {
    fn fail_n_times(n: u32) -> Self {
        Self {
            sent: Default::default(),
            failures_before_success: Mutex::new(n),
            permanent_failure: Mutex::new(false),
        }
    }

    fn permanent_failure() -> Self {
        Self {
            sent: Default::default(),
            failures_before_success: Mutex::new(0),
            permanent_failure: Mutex::new(true),
        }
    }
}

#[async_trait]
impl EmailSender for FakeSender {
    async fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), ApplicationError> {
        if *self.permanent_failure.lock().unwrap() {
            return Err(ApplicationError::EmailSendFailed);
        }

        let mut remaining = self.failures_before_success.lock().unwrap();
        if *remaining > 0 {
            *remaining -= 1;
            return Err(ApplicationError::EmailSendFailed);
        }

        self.sent
            .lock()
            .unwrap()
            .push((to.to_string(), subject.to_string(), body.to_string()));
        Ok(())
    }
}

#[tokio::test]
async fn worker_sends_queued_email() {
    let (queue, receiver) = InMemoryEmailQueue::new();
    let sender = Arc::new(FakeSender::default());

    let worker = tokio::spawn(run_worker(receiver, sender.clone(), 3, 10, 100));

    queue
        .enqueue(EmailQueueItem {
            to: "user@example.com".to_string(),
            subject: "Hello".to_string(),
            body: "World".to_string(),
        })
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    drop(queue);
    worker.await.unwrap();

    let sent = sender.sent.lock().unwrap();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].0, "user@example.com");
    assert_eq!(sent[0].1, "Hello");
    assert_eq!(sent[0].2, "World");
}

#[tokio::test]
async fn worker_retries_until_success() {
    let (queue, receiver) = InMemoryEmailQueue::new();
    let sender = Arc::new(FakeSender::fail_n_times(2));

    let worker = tokio::spawn(run_worker(receiver, sender.clone(), 3, 10, 100));

    queue
        .enqueue(EmailQueueItem {
            to: "retry@example.com".to_string(),
            subject: "Retry".to_string(),
            body: "Please".to_string(),
        })
        .await
        .unwrap();

    // Wait for two retries plus processing time.
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    drop(queue);
    worker.await.unwrap();

    assert_eq!(sender.sent.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn worker_gives_up_after_max_retries() {
    let (queue, receiver) = InMemoryEmailQueue::new();
    let sender = Arc::new(FakeSender::permanent_failure());

    let worker = tokio::spawn(run_worker(receiver, sender.clone(), 2, 10, 100));

    queue
        .enqueue(EmailQueueItem {
            to: "fail@example.com".to_string(),
            subject: "Fail".to_string(),
            body: "Always".to_string(),
        })
        .await
        .unwrap();

    // Wait for all retries.
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    drop(queue);
    worker.await.unwrap();

    assert_eq!(sender.sent.lock().unwrap().len(), 0);
}

#[tokio::test]
async fn worker_survives_send_failure() {
    let (queue, receiver) = InMemoryEmailQueue::new();
    let sender = Arc::new(FakeSender::permanent_failure());

    let worker = tokio::spawn(run_worker(receiver, sender.clone(), 0, 10, 100));

    queue
        .enqueue(EmailQueueItem {
            to: "fail@example.com".to_string(),
            subject: "Hello".to_string(),
            body: "World".to_string(),
        })
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    drop(queue);
    worker.await.unwrap();

    assert_eq!(sender.sent.lock().unwrap().len(), 0);
}
