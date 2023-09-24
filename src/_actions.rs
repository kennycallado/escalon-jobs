use escalon::tokio;
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

use crate::manager::{ContextTrait, EscalonJobsManager};
use crate::{EscalonJobStatus, EscalonJobTrait, NewEscalonJob};

impl<T: ContextTrait<T> + Clone + Send + Sync + 'static> EscalonJobsManager<T> {
    pub async fn create_job(
        &self,
        new_cron_job: impl EscalonJobTrait<T> + Into<NewEscalonJob> + Clone + Send + Sync + 'static,
    ) -> Uuid {
        let new_job = new_cron_job.clone().into();
        let manager = self.clone();

        let job = Job::new_async(new_job.schedule.as_str(), move |uuid, lock| {
            let new_job = new_cron_job.clone();
            let manager = manager.clone();

            Box::pin(async move {
                manager.status_handler(uuid, lock, new_job).await;
            })
        })
        .unwrap();

        let scheduler;
        {
            scheduler = self.scheduler.lock().unwrap().clone();
        }
        scheduler.add(job).await.unwrap()
    }

    async fn status_handler(
        &self,
        uuid: Uuid,
        mut lock: JobScheduler,
        new_cron_job: impl EscalonJobTrait<T> + Into<NewEscalonJob> + Clone + Send + Sync + 'static,
    ) {
        let new_job = new_cron_job.clone().into();
        let next_tick = lock.next_tick_for_job(uuid).await.unwrap().unwrap().naive_utc();

        let status =
            self.jobs.lock().unwrap().iter().find(|j| j.job_id == uuid).unwrap().status.clone();

        // let n_jobs = self.jobs.lock().unwrap().len();
        // println!("n_jobs: {}", n_jobs);

        match status {
            EscalonJobStatus::Scheduled => {
                if let Some(since) = new_job.since {
                    if since > next_tick {
                        return;
                    }
                }

                if let Some(job) = self.get_job(uuid).await {
                    self.update_status(uuid, EscalonJobStatus::Running);
                    self.context.0.update_job(&self.context.0, job).await;
                }
            }
            EscalonJobStatus::Running => {
                let manager = self.clone();

                tokio::task::spawn(async move {
                    let job = manager.get_job(uuid).await;
                    match job {
                        Some(job) => {
                            let es_job = new_cron_job
                                .run_job(manager.context.0.clone(), job.clone())
                                .await;
                            if es_job != job {
                                if let Some(job) = manager.get_job(uuid).await {
                                    manager.update_status(uuid, es_job.status.to_owned());
                                    manager.context.0.update_job(&manager.context.0, job).await;
                                }
                            }
                        }
                        None => {}
                    }
                });
            }
            EscalonJobStatus::Done | EscalonJobStatus::Failed => {
                //
                // let manager = self.clone();
                // let id = uuid;
                // tokio::task::spawn(async move {
                //     let scheduler;
                //     {
                //         scheduler = manager.scheduler.lock().unwrap().clone();
                //     }
                //     scheduler.remove(&id).await.unwrap();

                //     manager.jobs.lock().unwrap().retain(|j| j.job_id != id)
                // });
                //

                //
                //

                //
                // let scheduler;
                // {
                //     scheduler = self.scheduler.lock().unwrap().clone();
                // }
                // scheduler.remove(&uuid).await.unwrap();

                // self.jobs.lock().unwrap().retain(|j| j.job_id != uuid);
                //

                if let Some(job) = self.get_job(uuid).await {
                    self.context.0.update_job(&self.context.0, job).await;
                    self.remove_job(uuid);
                }
            }
        }

        if let Some(until) = new_job.until {
            if until < next_tick {
                if let Some(job) = self.get_job(uuid).await {
                    self.update_status(uuid, EscalonJobStatus::Done);
                    self.context.0.update_job(&self.context.0, job).await;
                }
            }
        }
    }

    fn update_status(&self, id: Uuid, status: EscalonJobStatus) {
        let jobs = self.jobs.clone();

        {
            let mut jobs = jobs.lock().unwrap();
            let job = jobs.iter_mut().find(|j| j.job_id == id).unwrap();
            job.status = status;
        }
    }
}
