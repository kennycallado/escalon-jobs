pub mod manager;

use async_trait::async_trait;
use chrono::NaiveDateTime;
use manager::Context;
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
pub trait EscalonJobTrait<T> {
    async fn run(&self, ctx: Context<T>, job: EscalonJob);
    async fn update_db(&self, job: &EscalonJob);
}

#[derive(Debug, Clone)]
pub struct NewEscalonJob {
    pub schedule: String,
    pub since: Option<NaiveDateTime>,
    pub until: Option<NaiveDateTime>,
}
