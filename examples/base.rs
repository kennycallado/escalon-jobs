use std::{net::IpAddr, time::Duration};

use async_trait::async_trait;
use chrono::NaiveDateTime;
use escalon_jobs::manager::{Context, EscalonJobsManager};
use escalon_jobs::{EscalonJob, EscalonJobStatus, EscalonJobTrait, NewEscalonJob};
use reqwest::Client;
use tokio::signal::unix::{signal, SignalKind};

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
impl EscalonJobTrait<Option<Client>> for NewAppJob {
    async fn run(&self, ctx: Context<Option<Client>>, mut job: EscalonJob) {
        let url = std::env::var("URL").unwrap_or("https://httpbin.org/status/200".to_string());
        let client = ctx.0.lock().unwrap().clone().unwrap();

        let req = client.get(url).send().await.unwrap();
        match req.status() {
            reqwest::StatusCode::OK => println!("{} - Status: OK", job.job_id),
            _ => {
                println!("{} - Status: {}", job.job_id, req.status());

                job.status = EscalonJobStatus::Failed;
                self.update_db(&job).await;
            }
        }

        // self.update_db(&job).await;
    }

    async fn update_db(&self, job: &EscalonJob) {
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
    let iden = std::env::var("HOSTNAME").unwrap_or("server".to_string());
    // config

    // let new_app_job_1 = NewAppJob {
    //     service: "test".to_owned(),
    //     route: "test".to_owned(),
    //     schedule: "0/8 * * * * *".to_owned(),
    //     since: None,
    //     until: None,
    // };

    // let new_app_job_2 = NewAppJob {
    //     service: "test".to_owned(),
    //     route: "test".to_owned(),
    //     schedule: "0/5 * * * * *".to_owned(),
    //     since: None,
    //     until: None,
    // };

    // start service
    let jm = EscalonJobsManager::<Client>::new();
    let jm = jm
        .set_id(iden)
        .set_addr(addr)
        .set_port(port)
        .set_context(Some(Client::new()))
        .build()
        .await;

    jm.init().await;
    // end service

    // call from handlers

    for i in 1..=100 {
        let new_app_job = NewAppJob {
            service: format!("test_{}", i),
            route: "test".to_owned(),
            schedule: "0/5 * * * * *".to_owned(),
            since: None,
            until: None,
        };

        jm.create_job(new_app_job).await;
    }

    // jm.create_job(new_app_job_1).await;
    // jm.create_job(new_app_job_2).await;
    // call from handlers

    // temp just to keep the server running
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });

    signal(SignalKind::terminate()).unwrap().recv().await;
    println!("Shutting down the server");
}
