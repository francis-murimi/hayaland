use application::errors::ApplicationError;
use application::ports::MediaStorage;
use async_trait::async_trait;
use std::path::PathBuf;
use uuid::Uuid;

pub struct LocalFileStorage {
    base_path: PathBuf,
    public_base_url: String,
}

impl LocalFileStorage {
    pub fn new(base_path: impl Into<PathBuf>, public_base_url: String) -> Self {
        Self {
            base_path: base_path.into(),
            public_base_url,
        }
    }

    fn ensure_base_dir(&self) -> Result<(), ApplicationError> {
        std::fs::create_dir_all(&self.base_path).map_err(|e| ApplicationError::MediaStorageFailed {
            message: format!("failed to create storage directory: {e}"),
        })
    }

    fn subdir(&self) -> String {
        let now = time::OffsetDateTime::now_utc();
        format!("{:04}/{:02}", now.year(), now.month() as u8)
    }
}

#[async_trait]
impl MediaStorage for LocalFileStorage {
    async fn store(
        &self,
        content: &[u8],
        extension: Option<&str>,
    ) -> Result<String, ApplicationError> {
        self.ensure_base_dir()?;
        let subdir = self.subdir();
        let dir = self.base_path.join(&subdir);
        std::fs::create_dir_all(&dir).map_err(|e| ApplicationError::MediaStorageFailed {
            message: format!("failed to create subdirectory: {e}"),
        })?;

        let filename = match extension {
            Some(ext) => format!("{}.{}", Uuid::now_v7(), ext),
            None => Uuid::now_v7().to_string(),
        };
        let relative_path = format!("{}/{}", subdir, filename);
        let full_path = self.base_path.join(&relative_path);

        tokio::fs::write(&full_path, content).await.map_err(|e| {
            ApplicationError::MediaStorageFailed {
                message: format!("failed to write file: {e}"),
            }
        })?;

        Ok(relative_path)
    }

    async fn delete(&self, path: &str) -> Result<(), ApplicationError> {
        let full_path = self.base_path.join(path);
        if full_path.exists() {
            tokio::fs::remove_file(&full_path).await.map_err(|e| {
                ApplicationError::MediaStorageFailed {
                    message: format!("failed to delete file: {e}"),
                }
            })?;
        }
        Ok(())
    }

    async fn public_url(&self, path: &str) -> Result<String, ApplicationError> {
        let url = if self.public_base_url.ends_with('/') {
            format!("{}{}", self.public_base_url, path)
        } else {
            format!("{}/{}", self.public_base_url, path)
        };
        Ok(url)
    }
}
