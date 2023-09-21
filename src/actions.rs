use uuid::Uuid;
use escalon::tokio as tokio;

use crate::manager::{EscalonJobsManager, ContextTrait};
use crate::{EscalonJob, EscalonJobTrait, NewEscalonJob};

impl<T: ContextTrait<T> + Clone + Send + Sync + 'static> EscalonJobsManager<T> {
    pub async fn get_job(&self, id: Uuid) -> Option<EscalonJob> {
        self.jobs.lock().unwrap().iter().find(|j| j.job_id == id).cloned()
    }

    pub async fn get_jobs(&self) -> Vec<EscalonJob> {
        self.jobs.lock().unwrap().to_owned()
    }

    pub async fn add_job(
        &self,
        new_cron_job: impl EscalonJobTrait<T> + Into<NewEscalonJob> + Clone + Send + Sync + 'static,
    ) -> EscalonJob {
        let new_job = new_cron_job.clone().into();
        let job_id = self.create_job(new_cron_job).await;

        let es_job = EscalonJob {
            job_id,
            status: Default::default(),
            schedule: new_job.schedule,
            since: new_job.since,
            until: new_job.until,
        };

        self.jobs.lock().unwrap().push(es_job.clone());

        es_job
    }

    pub fn remove_job(&self, id: Uuid) {
        let manager = self.clone();
        // let jobs = self.jobs.clone();
        // let scheduler = self.scheduler.clone();

        tokio::task::spawn(async move {
            let scheduler;
            {
                scheduler = manager.scheduler.lock().unwrap().clone();
            }
            match scheduler.remove(&id).await {
                Ok(_) => manager.jobs.lock().unwrap().retain(|j| j.job_id != id) ,
                Err(e) => println!("Error removing job: {}", e),
            }
        });
        // self.context.0.update_job(&self.context.0, self.get_job(uuid).await).await;

        // let scheduler;
        // {
        //     scheduler = self.scheduler.lock().unwrap().clone();
        // }
        // scheduler.remove(&id).await.unwrap();

        // self.jobs.lock().unwrap().retain(|j| j.job_id != id);
    }

    pub async fn update_job(
        &self,
        id: Uuid,
        new_cron_job: impl EscalonJobTrait<T> + Into<NewEscalonJob> + Clone + Send + Sync + 'static,
    ) -> EscalonJob {
        let new_job = new_cron_job.clone().into();
        let jobs = self.jobs.clone();

        let mut job;
        {
            job = jobs.lock().unwrap().iter().find(|j| j.job_id == id).unwrap().clone();
        }

        job.schedule = new_job.schedule;
        job.since = new_job.since;
        job.until = new_job.until;

        let scheduler;
        {
            scheduler = self.scheduler.lock().unwrap().clone();
        }
        scheduler.remove(&id).await.unwrap();

        self.create_job(new_cron_job).await;

        job.clone()
    }
}
