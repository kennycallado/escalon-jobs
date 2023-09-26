#![allow(dead_code)]
use std::net::IpAddr;
use std::time::Duration;

use async_trait::async_trait;
use chrono::NaiveDateTime;
use escalon::tokio;
use escalon_jobs::manager::{ContextTrait, EscalonJobsManager, EscalonJobsManagerTrait};
use escalon_jobs::{EscalonJob, EscalonJobStatus, EscalonJobTrait, NewEscalonJob};
use rand::Rng;
use reqwest::Client;
use tokio::signal::unix::{signal, SignalKind};

#[derive(Debug, Clone)]
pub struct Context<T>(pub T);
impl Context<Client> {
    pub fn new() -> Self {
        Context(Client::new())
    }
}

#[async_trait]
impl ContextTrait<Context<Client>> for Context<Client> {
    async fn update_job(&self, Context(_ctx): &Context<Client>, _job: EscalonJob) {
        // println!("Job: {:?} - updating to db", job);
        println!("updating to db");
        escalon::tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}

pub struct Manager;
#[async_trait]
impl EscalonJobsManagerTrait<Context<Client>> for Manager {
    async fn take_jobs(
        &self,
        _manager: &EscalonJobsManager<Context<Client>>,
        from_client: String,
        start_at: usize,
        n_jobs: usize,
    ) -> Result<Vec<String>, ()> {
        println!("{} - {} - {}", from_client, start_at, n_jobs);
        // access DB

        Ok(Vec::new())
    }

    async fn drop_jobs(
        &self,
        _manager: &EscalonJobsManager<Context<Client>>,
        jobs: Vec<String>,
    ) -> Result<(), ()> {
        println!("Drop jobs: {:?}", jobs);

        Ok(())
    }
}

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
impl EscalonJobTrait<Context<Client>> for NewAppJob {
    async fn run_job(
        &self,
        Context(client): &Context<Client>,
        mut job: EscalonJob,
    ) -> EscalonJob {
        let url = std::env::var("URL").unwrap_or("https://httpbin.org/status/200".to_string());
        let req = client.get(url).send().await.unwrap();

        match req.status() {
            reqwest::StatusCode::OK => {
                println!("{} - Status: OK", job.job_id)
            }
            _ => {
                println!("{} - Status: {}", job.job_id, req.status());

                job.status = EscalonJobStatus::Failed;
            }
        }

        job
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

    let context = Context(Client::builder().timeout(Duration::from_secs(5)).build().unwrap());

    let manager = Manager;
    // start service
    let jm = EscalonJobsManager::new(context);
    let jm = jm.set_id(iden).set_addr(addr).set_port(port).set_functions(manager).build().await;

    // let jm = EscalonJobsManager::new(Context(None));
    // let jm = jm.set_id(iden).set_addr(addr).set_port(port).build().await;

    jm.init().await;
    // end service

    // call from handlers
    for i in 1..=100 {
        let sec = rand::thread_rng().gen_range(1..6);
        let schedule = format!("0/{} * * * * *", sec);
        // let schedule = "0/5 * * * * *".to_owned();

        let new_app_job = NewAppJob {
            service: format!("test_{}", i),
            route: "test".to_owned(),
            schedule,
            since: None,
            until: None,
        };

        jm.add_job(new_app_job).await;
    }

    signal(SignalKind::terminate()).unwrap().recv().await;
    println!("Shutting down the server");
}
