use async_trait::async_trait;
pub use escalon::EscalonClient;
use escalon::{Escalon, EscalonTrait};
use std::collections::HashMap;
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
pub struct Service(IpAddr);
pub struct NoService;

#[derive(Clone)]
pub struct Port(u16);
pub struct NoPort;

pub struct Functions<T>(pub Arc<dyn EscalonJobsManagerTrait<T>>);
pub struct NoFunctions;

#[derive(Clone)]
pub struct Context<T>(pub T);

#[async_trait]
pub trait ContextTrait<T> {
    async fn update_job(&self, ctx: &T, job: EscalonJob);
}

pub struct EscalonJobsManagerBuilder<I, A, S, P, C, F> {
    id: I,
    addr: A,
    svc: S,
    port: P,
    context: C,
    functions: F,
}

impl<I, A, S, P, C, F> EscalonJobsManagerBuilder<I, A, S, P, C, F> {
    pub fn set_id(self, id: String) -> EscalonJobsManagerBuilder<Id, A, S, P, C, F> {
        EscalonJobsManagerBuilder {
            id: Id(id),
            addr: self.addr,
            svc: self.svc,
            port: self.port,
            context: self.context,
            functions: self.functions,
        }
    }

    pub fn set_addr(self, addr: IpAddr) -> EscalonJobsManagerBuilder<I, Addr, S, P, C, F> {
        EscalonJobsManagerBuilder {
            id: self.id,
            addr: Addr(addr),
            svc: self.svc,
            port: self.port,
            context: self.context,
            functions: self.functions,
        }
    }

    pub fn set_svc(self, svc: IpAddr) -> EscalonJobsManagerBuilder<I, A, Service, P, C, F> {
        EscalonJobsManagerBuilder {
            id: self.id,
            addr: self.addr,
            svc: Service(svc),
            port: self.port,
            context: self.context,
            functions: self.functions,
        }
    }

    pub fn set_port(self, port: u16) -> EscalonJobsManagerBuilder<I, A, S, Port, C, F> {
        EscalonJobsManagerBuilder {
            id: self.id,
            addr: self.addr,
            svc: self.svc,
            port: Port(port),
            context: self.context,
            functions: self.functions,
        }
    }

    pub fn set_functions<T: ContextTrait<T>>(
        self,
        functions: impl EscalonJobsManagerTrait<T> + Send + Sync + 'static,
    ) -> EscalonJobsManagerBuilder<I, A, S, P, C, Functions<T>> {
        EscalonJobsManagerBuilder {
            id: self.id,
            addr: self.addr,
            svc: self.svc,
            port: self.port,
            context: self.context,
            functions: Functions(Arc::new(functions)),
        }
    }
}

impl<C: ContextTrait<C>>
    EscalonJobsManagerBuilder<Id, Addr, Service, Port, Context<C>, Functions<C>>
{
    pub async fn build(self) -> EscalonJobsManager<C> {
        let jobs = Arc::new(Mutex::new(Vec::new()));
        let scheduler = JobScheduler::new().await.unwrap();

        EscalonJobsManager {
            scheduler: Arc::new(Mutex::new(scheduler)),
            jobs,
            context: self.context.0,
            functions: self.functions.0,
            id: self.id,
            addr: self.addr,
            port: self.port,
            clients: None,
        }
    }
}

#[derive(Clone)]
pub struct EscalonJobsManager<T: ContextTrait<T>> {
    pub scheduler: Arc<Mutex<JobScheduler>>,
    pub jobs: Arc<Mutex<Vec<EscalonJob>>>,
    pub context: T,
    pub functions: Arc<dyn EscalonJobsManagerTrait<T>>,
    pub clients: Option<Arc<Mutex<HashMap<String, EscalonClient>>>>,
    id: Id,
    addr: Addr,
    port: Port,
    // clients: Arc<Mutex<HashMap<String, EscalonClient>>>,
    // pub clients: Option<Arc<Mutex<HashMap<String, EscalonClient>>>>,
}

#[async_trait]
pub trait EscalonJobsManagerTrait<T: ContextTrait<T>>: Send + Sync + 'static {
    async fn take_jobs(
        &self,
        manager: &EscalonJobsManager<T>,
        from_client: String,
        start_at: usize,
        n_jobs: usize,
    ) -> Result<Vec<String>, ()>;
    async fn drop_jobs(
        &self,
        manager: &EscalonJobsManager<T>,
        jobs: Vec<String>,
    ) -> Result<(), ()>;
}

#[async_trait]
impl<T: ContextTrait<T> + Clone + Send + Sync + 'static> EscalonTrait
    for EscalonJobsManager<T>
{
    fn count(&self) -> usize {
        self.jobs.lock().unwrap().len()
    }

    async fn take_jobs(
        &self,
        from_client: String,
        start_at: usize,
        n_jobs: usize,
    ) -> Result<Vec<String>, ()> {
        self.functions.take_jobs(self, from_client, start_at, n_jobs).await
    }

    async fn drop_jobs(&self, jobs: Vec<String>) -> Result<(), ()> {
        self.functions.drop_jobs(self, jobs).await
    }
}

impl<T: ContextTrait<T> + Clone + Send + Sync + 'static> EscalonJobsManager<T> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        context: T,
    ) -> EscalonJobsManagerBuilder<NoId, NoAddr, NoService, NoPort, Context<T>, NoFunctions>
    {
        EscalonJobsManagerBuilder {
            id: NoId,
            addr: NoAddr,
            svc: NoService,
            port: NoPort,
            functions: NoFunctions,
            context: Context(context),
        }
    }

    pub async fn init(&mut self) {
        let manager = self.clone();

        let mut udp_server = Escalon::new()
            .set_id(&self.id.0)
            .set_addr(self.addr.0)
            .set_port(self.port.0)
            .set_manager(manager.clone())
            .build()
            .await;

        {
            let scheduler = self.scheduler.lock().unwrap().clone();
            scheduler.start().await.unwrap();
        }

        self.clients = Some(Arc::clone(&udp_server.clients));

        udp_server.listen().await
    }
}
