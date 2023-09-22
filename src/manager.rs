use async_trait::async_trait;
use escalon::Escalon;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use tokio_cron_scheduler::JobScheduler;

use crate::EscalonJob;

#[derive(Clone)]
pub struct Id(String);
pub struct NoId;

#[derive(Clone)]
pub struct Addr(IpAddr);
pub struct NoAddr;

#[derive(Clone)]
pub struct Port(u16);
pub struct NoPort;

#[derive(Clone)]
pub struct Context<T: ContextTrait<T>>(pub T);


#[async_trait]
pub trait ContextTrait<T: ContextTrait<T>> {
    async fn update_job(&self, context: &T, job: EscalonJob);
    async fn take_jobs(&self, manager: &EscalonJobsManager<T>, from: String, start_at: usize, n_jobs: usize);
}

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

impl<C: ContextTrait<C>> EscalonJobsManagerBuilder<Id, Addr, Port, Context<C>> {
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

#[derive(Clone)]
pub struct EscalonJobsManager<T: ContextTrait<T>> {
    pub scheduler: Arc<Mutex<JobScheduler>>,
    pub jobs: Arc<Mutex<Vec<EscalonJob>>>,
    pub context: Context<T>,
    id: Id,
    addr: Addr,
    port: Port,
}

impl<T: ContextTrait<T> + Clone + Send + Sync + 'static> EscalonJobsManager<T> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(context: T) -> EscalonJobsManagerBuilder<NoId, NoAddr, NoPort, Context<T>> {
        EscalonJobsManagerBuilder {
            id: NoId,
            addr: NoAddr,
            port: NoPort,
            context: Context(context),
        }
}

    fn spawn_take_jobs(&self, from: String, start_at: usize, n_jobs: usize) {
        let manager = self.clone();

        escalon::tokio::spawn(async move {
            // TODO
            // let news = manager.context.0.take_jobs(&manager.context.0, from, start_at, n_jobs, |new| { manager.add_job(new); }).await;

            manager.context.0.take_jobs(&manager, from, start_at, n_jobs).await

            // self.context.0.take_jobs(from, start_at, n_jobs).await;
        });
    }

    pub async fn init(&self) {
        let jobs_one = self.jobs.clone();
        let manager = self.clone();

        let mut udp_server = Escalon::new()
            .set_id(&self.id.0)
            .set_addr(self.addr.0)
            .set_port(self.port.0)
            .set_count_jobs(move || jobs_one.lock().unwrap().len())
            .set_take_jobs(move |from, start_at, n_jobs| {
                manager.spawn_take_jobs(from.to_string(), start_at, n_jobs);
            })
            // .set_take_jobs(move |from, start_at, n_jobs| { context.0.take_jobs(from, start_at, n_jobs); })
            .build()
            .await;

        {
            let scheduler = self.scheduler.lock().unwrap().clone();
            scheduler.start().await.unwrap();
        }

        udp_server.listen().await.unwrap()
    }
}
