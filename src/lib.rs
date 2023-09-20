mod _actions;
mod actions;
pub mod manager;

use async_trait::async_trait;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub enum EscalonJobStatus {
    #[default]
    Scheduled,
    Running,
    Done,
    Failed,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct EscalonJob {
    pub job_id: Uuid,
    pub status: EscalonJobStatus,
    pub schedule: String,
    pub since: Option<NaiveDateTime>,
    pub until: Option<NaiveDateTime>,
}

#[async_trait]
pub trait EscalonJobTrait<T> {
    async fn run_job(&self, mut job: EscalonJob, ctx: T) -> EscalonJob;
}

#[derive(Debug, Clone)]
pub struct NewEscalonJob {
    pub schedule: String,
    pub since: Option<NaiveDateTime>,
    pub until: Option<NaiveDateTime>,
}
