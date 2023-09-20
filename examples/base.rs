use std::net::IpAddr;

use escalon::tokio as tokio;

use async_trait::async_trait;
use chrono::NaiveDateTime;
use escalon_jobs::manager::{EscalonJobsManager, ContextTrait};
use escalon_jobs::{EscalonJob, EscalonJobTrait, NewEscalonJob, EscalonJobStatus};
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

impl ContextTrait<Context<Client>> for Context<Client> {
    fn update_job(&self, _job: &EscalonJob, Context(_ctx): &Context<Client>) {
        let _ = _ctx.get("https://httpbin.org/status/200");

        println!("Job: {:?} - updating to db", _job);
    }

    fn take_jobs(&self, _from: &str, _start_at: usize, _n_jobs: usize) {
        // println!("Take jobs from: {} start_at: {} - n_jobs: {}", from, start_at, n_jobs);
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
    async fn run_job(&self, mut job: EscalonJob, Context(ctx): Context<Client>) -> EscalonJob {
        let url = std::env::var("URL").unwrap_or("https://httpbin.org/status/200".to_string());
        let req = ctx.get(url).send().await.unwrap();

        match req.status() {
            reqwest::StatusCode::OK => {
                println!("{} - Status: OK", job.job_id)
            },
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

    let context = Context(Client::new());

    // start service
    let jm = EscalonJobsManager::new(context);
    let jm = jm.set_id(iden).set_addr(addr).set_port(port).build().await;
    //
    // let jm = EscalonJobsManager::new(Context(None));
    // let jm = jm.set_id(iden).set_addr(addr).set_port(port).build().await;

    jm.init().await;
    // end service

    // call from handlers
    for i in 1..=200 {
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
    // call from handlers

    signal(SignalKind::terminate()).unwrap().recv().await;
    println!("Shutting down the server");
}
