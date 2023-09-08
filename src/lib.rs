pub mod manager;

use async_trait::async_trait;
use chrono::NaiveDateTime;
use std::sync::{Arc, Mutex};
pub use tokio_cron_scheduler::JobScheduler;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct EscalonJob {
    pub job_id: Uuid,
    pub status: String,
    pub schedule: String,
    pub since: Option<NaiveDateTime>,
    pub until: Option<NaiveDateTime>,
}

#[async_trait]
pub trait EscalonJobTrait {
    async fn run(&self, job: EscalonJob);
    async fn update_db(&self, job: &EscalonJob);
}

#[derive(Debug, Clone)]
pub struct NewEscalonJob {
    pub schedule: String,
    pub since: Option<NaiveDateTime>,
    pub until: Option<NaiveDateTime>,
}
