use escalon::Escalon;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::{EscalonJob, EscalonJobStatus, EscalonJobTrait, NewEscalonJob};

pub struct NoId;
pub struct Id(String);

pub struct NoAddr;
pub struct Addr(IpAddr);

pub struct NoPort;
pub struct Port(u16);

#[derive(Clone)]
pub struct Context<T>(pub T);

pub struct EscalonJobsManagerBuilder<I, A, P, C> {
    id: I,
    addr: A,
    port: P,
    context: C,
}

impl<I, A, P, C> EscalonJobsManagerBuilder<I, A, P, C> {
    pub fn set_id(self, id: String) -> EscalonJobsManagerBuilder<Id, A, P, C> {
        EscalonJobsManagerBuilder {
            id: Id(id),
            addr: self.addr,
            port: self.port,
            context: self.context,
        }
    }

    pub fn set_addr(self, addr: IpAddr) -> EscalonJobsManagerBuilder<I, Addr, P, C> {
        EscalonJobsManagerBuilder {
            id: self.id,
            addr: Addr(addr),
            port: self.port,
            context: self.context,
        }
    }

    pub fn set_port(self, port: u16) -> EscalonJobsManagerBuilder<I, A, Port, C> {
        EscalonJobsManagerBuilder {
            id: self.id,
            addr: self.addr,
            port: Port(port),
            context: self.context,
        }
    }
}

impl<C> EscalonJobsManagerBuilder<Id, Addr, Port, Context<C>> {
    pub async fn build(self) -> EscalonJobsManager<C> {
        let jobs = Arc::new(Mutex::new(Vec::new()));
        let scheduler = JobScheduler::new().await.unwrap();

        EscalonJobsManager {
            scheduler: Arc::new(Mutex::new(scheduler)),
            jobs,
            context: self.context,
            id: self.id,
            addr: self.addr,
            port: self.port,
        }
    }
}

pub struct EscalonJobsManager<T> {
    scheduler: Arc<Mutex<JobScheduler>>,
    jobs: Arc<Mutex<Vec<EscalonJob>>>,
    context: Context<T>,
    id: Id,
    addr: Addr,
    port: Port,
}

impl<T: Clone + Send + Sync + 'static> EscalonJobsManager<T> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        context: Context<T>,
    ) -> EscalonJobsManagerBuilder<NoId, NoAddr, NoPort, Context<T>> {
        EscalonJobsManagerBuilder {
            id: NoId,
            addr: NoAddr,
            port: NoPort,
            context,
        }
    }

    pub async fn init(&self) {
        let jobs = self.jobs.clone();
        let mut udp_server = Escalon::<EscalonJob>::new()
            .set_id(&self.id.0)
            .set_addr(self.addr.0)
            .set_port(self.port.0)
            .build(jobs)
            .await
            .unwrap();

        {
            let scheduler = self.scheduler.lock().unwrap().clone();
            scheduler.start().await.unwrap();
        }

        udp_server.listen().await.unwrap()
    }

    pub async fn create_job(
        &self,
        new_cron_job: impl EscalonJobTrait<T> + Into<NewEscalonJob> + Clone + Send + Sync + 'static,
    ) -> EscalonJob {
        let cloned = new_cron_job.clone().into();
        let ctx = self.context.clone();
        let jobs = self.jobs.clone();
        let new_job = new_cron_job.clone();

        let job =
            Job::new_async(new_job.into().schedule.clone().as_str(), move |uuid, lock| {
                let jobs = jobs.clone();
                let new_cron_job = new_cron_job.clone();
                let ctx = ctx.clone();

                Box::pin(async move {
                    let status = jobs
                        .lock()
                        .unwrap()
                        .iter()
                        .find(|j| j.job_id == uuid)
                        .unwrap()
                        .status
                        .clone();

                    match status {
                        EscalonJobStatus::Scheduled => {
                            // check things like since and until
                            // to change state to active
                            jobs.lock()
                                .unwrap()
                                .iter_mut()
                                .find(|j| j.job_id == uuid)
                                .unwrap()
                                .status = EscalonJobStatus::Running;

                            let job = jobs
                                .lock()
                                .unwrap()
                                .iter()
                                .find(|j| j.job_id == uuid)
                                .unwrap()
                                .clone();

                            new_cron_job.update_db(&job).await;
                        }
                        EscalonJobStatus::Running => {
                            let job = jobs
                                .lock()
                                .unwrap()
                                .iter()
                                .find(|j| j.job_id == uuid)
                                .unwrap()
                                .clone();

                            new_cron_job.run(job, ctx.clone()).await;
                        }
                        EscalonJobStatus::Done | EscalonJobStatus::Failed => {
                            lock.remove(&uuid).await.unwrap();
                        }
                    }
                })
            })
            .unwrap();

        let job_id;
        {
            let scheduler = self.scheduler.lock().unwrap().clone();
            job_id = scheduler.add(job).await.unwrap();
        }

        let cron_job = EscalonJob {
            job_id,
            status: EscalonJobStatus::Scheduled,
            schedule: cloned.schedule,
            since: cloned.since,
            until: cloned.until,
        };

        self.jobs.lock().unwrap().push(cron_job.clone());

        cron_job
    }
}
