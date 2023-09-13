pub mod manager;

use async_trait::async_trait;
use chrono::NaiveDateTime;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum EscalonJobStatus {
    #[default]
    Scheduled,
    Running,
    Done,
    Failed,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct EscalonJob {
    pub job_id: Uuid,
    pub status: EscalonJobStatus,
    pub schedule: String,
    pub since: Option<NaiveDateTime>,
    pub until: Option<NaiveDateTime>,
}

/// T is the context type that will be passed
/// should be an Arc<Mutex<_>> of the context
/// that will be used in the job
#[async_trait]
pub trait EscalonJobTrait<T> {
    async fn run(&self, ctx: T, job: EscalonJob);
    async fn update_db(&self, job: &EscalonJob);
}

#[derive(Debug, Clone)]
pub struct NewEscalonJob {
    pub schedule: String,
    pub since: Option<NaiveDateTime>,
    pub until: Option<NaiveDateTime>,
}
