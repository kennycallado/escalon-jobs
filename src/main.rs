mod manager;

use std::{
    net::IpAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use chrono::NaiveDateTime;
use escalon_jobs::{EscalonJob, EscalonJobTrait, NewEscalonJob};
use manager::JobManager;
use tokio_cron_scheduler::JobScheduler;
use uuid::Uuid;

pub struct AppJob {
    pub id: usize,
    pub service: String,
    pub route: String,
    pub cron_job: EscalonJob,
}

#[derive(Debug, Clone)]
pub struct NewAppJob {
    pub service: String,
    pub route: String,
    pub schedule: String,
    pub since: Option<NaiveDateTime>,
    pub until: Option<NaiveDateTime>,
}

// Also gives Into<_> for free
impl From<NewAppJob> for NewEscalonJob {
    fn from(job: NewAppJob) -> Self {
        NewEscalonJob {
            schedule: job.schedule,
            since: job.since,
            until: job.until,
        }
    }
}


#[async_trait]
impl EscalonJobTrait for NewAppJob {
    async fn run(
        &self,
        job_id: &Uuid,
        _lock: &JobScheduler,
        jobs: Arc<Mutex<Vec<EscalonJob>>>,
    ) {
        // let next_tick = lock.next_tick_for_job(job_id.clone()).await.unwrap().unwrap().naive_utc();

        let _status =
            jobs.lock().unwrap().iter().find(|j| j.job_id == *job_id).unwrap().status.clone();
        // println!("Job: {} - {}", job_id, status);

        jobs.lock().unwrap().iter_mut().find(|j| j.job_id == *job_id).unwrap().status =
            "running".to_owned();
        // println!("Job: {} - {}", job_id, status);

        jobs.lock().unwrap().iter_mut().find(|j| j.job_id == *job_id).unwrap().status =
            "active".to_owned();
        // println!("Job: {} - {}", job_id, status);

        // println!("Job {} started", job_id);
        // println!("Jobs {:?}", jobs.lock().unwrap());
        // println!("Next tick {:?}", next_tick);

        let job = jobs.lock().unwrap().iter().find(|j| j.job_id == *job_id).unwrap().clone();
        self.update_db(job).await;
    }

    async fn update_db(&self, job: EscalonJob) {
        tokio::time::sleep(Duration::from_secs(1)).await;
        println!("Job: {:?} - updating to db", job);
    }
}

#[tokio::main]
async fn main() {
    // config
    let addr =
        std::env::var("ADDR").unwrap_or("0.0.0.0".to_string()).parse::<IpAddr>().unwrap();
    let port = std::env::var("PORT").unwrap_or("65056".to_string()).parse::<u16>().unwrap();
    // config

    let new_app_job_1 = NewAppJob {
        service: "test".to_owned(),
        route: "test".to_owned(),
        schedule: "0/8 * * * * *".to_owned(),
        since: None,
        until: None,
    };

    let new_app_job_2 = NewAppJob {
        service: "test".to_owned(),
        route: "test".to_owned(),
        schedule: "0/5 * * * * *".to_owned(),
        since: None,
        until: None,
    };

    // start service
    let jm = JobManager::new();
    let jm = jm.set_addr(addr).set_port(port).build().await;

    jm.init().await;
    // end service

    // call from handlers
    jm.create_job(new_app_job_1).await;
    jm.create_job(new_app_job_2).await;
    // call from handlers

    tokio::time::sleep(Duration::from_secs(60)).await;
}
