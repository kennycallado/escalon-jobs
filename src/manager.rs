use escalon::Escalon;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::{EscalonJob, EscalonJobTrait, NewEscalonJob};

pub struct NoId;
pub struct Id(String);

pub struct NoAddr;
pub struct Addr(IpAddr);

pub struct NoPort;
pub struct Port(u16);

pub struct EscalonJobsManagerBuilder<I, A, P> {
    pub id: I,
    pub addr: A,
    pub port: P,
}

impl<I, A, P> EscalonJobsManagerBuilder<I, A, P> {
    pub fn set_id(self, id: String) -> EscalonJobsManagerBuilder<Id, A, P> {
        EscalonJobsManagerBuilder {
            id: Id(id),
            addr: self.addr,
            port: self.port,
        }
    }

    pub fn set_addr(self, addr: IpAddr) -> EscalonJobsManagerBuilder<I, Addr, P> {
        EscalonJobsManagerBuilder {
            id: self.id,
            addr: Addr(addr),
            port: self.port,
        }
    }

    pub fn set_port(self, port: u16) -> EscalonJobsManagerBuilder<I, A, Port> {
        EscalonJobsManagerBuilder {
            id: self.id,
            addr: self.addr,
            port: Port(port),
        }
    }
}

impl EscalonJobsManagerBuilder<Id, Addr, Port> {
    pub async fn build(self) -> EscalonJobsManager {
        let scheduler = JobScheduler::new().await.unwrap();
        let jobs = Arc::new(Mutex::new(Vec::new()));

        // scheduler.start().await.unwrap();

        EscalonJobsManager {
            scheduler: Arc::new(Mutex::new(scheduler)),
            jobs,
            id: self.id,
            addr: self.addr,
            port: self.port,
        }
    }
}

pub struct EscalonJobsManager {
    scheduler: Arc<Mutex<JobScheduler>>,
    jobs: Arc<Mutex<Vec<EscalonJob>>>,
    id: Id,
    addr: Addr,
    port: Port,
}

impl EscalonJobsManager {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> EscalonJobsManagerBuilder<NoId, NoAddr, NoPort> {
        EscalonJobsManagerBuilder {
            id: NoId,
            addr: NoAddr,
            port: NoPort,
        }
    }

    pub async fn init(&self) {
        let scheduler;
        {
            scheduler = self.scheduler.lock().unwrap().clone();
        }
        scheduler.start().await.unwrap();

        let jobs = self.jobs.clone();

        let mut udp_server = Escalon::new()
            .set_id(&self.id.0)
            .set_addr(self.addr.0)
            .set_port(self.port.0)
            .set_count(move || jobs.lock().unwrap().len())
            .build()
            .await.unwrap();

        udp_server.listen().await.unwrap()
    }

    pub async fn create_job(
        &self,
        new_cron_job: impl EscalonJobTrait + Into<NewEscalonJob> + Clone + Send + Sync + 'static,
    ) -> EscalonJob {
        let new_job = new_cron_job.clone();
        let cloned = new_cron_job.clone().into();

        let jobs = self.jobs.clone();

        let job = Job::new_async(
            new_job.into().schedule.clone().as_str(),
            move |uuid, lock| {
                let jobs = jobs.clone();
                let new_cron_job = new_cron_job.clone();

                Box::pin(async move {
                    // while the status is pending not run the job
                    let status = jobs
                        .lock()
                        .unwrap()
                        .iter()
                        .find(|j| j.job_id == uuid)
                        .unwrap()
                        .status
                        .clone();

                    match status.as_str() {
                        "pending" => {
                            // check things like since and until
                            // to change state to active
                            println!("Job: {} - pending", uuid);
                            jobs.lock()
                                .unwrap()
                                .iter_mut()
                                .find(|j| j.job_id == uuid)
                                .unwrap()
                                .status = String::from("active");
                        }
                        "active" => {
                            new_cron_job.run(&uuid, &lock, jobs).await;
                        }
                        "done" => {
                            lock.remove(&uuid).await.unwrap();
                        }
                        _ => {}
                    }
                })
            },
        )
        .unwrap();

        let scheduler;
        {
            scheduler = self.scheduler.lock().unwrap().clone();
        }
        let job_id = scheduler.add(job).await.unwrap();

        // let job_id = self.scheduler.lock().unwrap().add(job).await.unwrap();

        let cron_job = EscalonJob {
            job_id,
            status: String::from("pending"),
            schedule: cloned.schedule,
            since: cloned.since,
            until: cloned.until,
        };

        self.jobs.lock().unwrap().push(cron_job.clone());

        cron_job
    }
}
