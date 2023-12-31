mod _actions;
mod actions;
pub mod manager;

use async_trait::async_trait;
use chrono::NaiveDateTime;
use uuid::Uuid;

#[derive(Clone, Debug, Default, PartialEq)]
pub enum EscalonJobStatus {
    #[default]
    Scheduled,
    Running,
    Done,
    Failed,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EscalonJob {
    pub job_id: Uuid,
    pub status: EscalonJobStatus,
    pub schedule: String,
    pub since: Option<NaiveDateTime>,
    pub until: Option<NaiveDateTime>,
}

#[async_trait]
pub trait EscalonJobTrait<T> {
    async fn run_job(&self, ctx: &T, mut job: EscalonJob) -> EscalonJob;
}

#[derive(Debug, Clone)]
pub struct NewEscalonJob {
    pub schedule: String,
    pub since: Option<NaiveDateTime>,
    pub until: Option<NaiveDateTime>,
}
