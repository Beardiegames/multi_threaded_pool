use std::{
    sync::{Arc, Mutex}, 
    thread::{self}
};

mod tests;
mod clusters;
pub use clusters::{ClusterPool, Cluster};


pub type ThreadIndex = usize;
pub type ClusterIndex = usize;

pub type OpperationHandler<PoolItem, Parameters> = fn(ThreadIndex, &mut ClusterPool<PoolItem>, &mut Parameters);
pub type SetupHandler<PoolItem, Parameters> = fn(&mut Cluster<PoolItem>, &mut Parameters);


pub struct ThreadPool<PoolItem, Parameters>
    where   PoolItem: Default + Clone + Send +'static, 
            Parameters: Send + 'static
{
    pub cluster_capacity: u32,
    pub run_handle: Arc<Mutex<bool>>,
    pub clusters: ClusterPool<PoolItem>,
    pub parameters: Arc<Mutex<Parameters>>,
}

impl<PoolItem, Parameters> ThreadPool<PoolItem, Parameters>
    where   PoolItem: Default + Clone + Send +'static, 
            Parameters: Send + 'static
{
    pub fn new(cluster_count: u8, cluster_size: u32, parameters: Parameters) -> Self {
        ThreadPool { 
            cluster_capacity: cluster_size,
            run_handle: Arc::new(Mutex::new(false)),
            clusters: ClusterPool::new(cluster_count, cluster_size),
            parameters: Arc::new(Mutex::new(parameters)),
        }
    }

    pub fn clusters(&mut self) -> &mut ClusterPool<PoolItem> {
        &mut self.clusters
    }

    pub fn parameters(&self) -> Arc<Mutex<Parameters>> {
        Arc::clone(&self.parameters)
    }

    pub fn start (
        &mut self, 
        setup: SetupHandler<PoolItem, Parameters>, 
        opperation: OpperationHandler<PoolItem, Parameters>,
    ) 
        where   PoolItem: Default + Clone + Send +'static,
                Parameters: Send + 'static,
    {
        {
            let handle = Arc::clone(&self.run_handle);
            let mut running = handle.lock().unwrap();

            if *running { return; } 
            *running = true;
        }

        let clusters = &self.clusters;

        for i in 0..clusters.len() {
            let run_handle = Arc::clone(&self.run_handle);
            let param_handle = Arc::clone(&self.parameters);
            let mut clusters = self.clusters.clone();

            thread::spawn(move || {
                {
                    (setup)(
                        &mut *clusters.arc_cluster(i).lock().unwrap(), 
                        &mut *param_handle.lock().unwrap()
                    );
                }

                'active: loop {
                    {
                        if !*run_handle.lock().unwrap() { break 'active; }
                    } {
                        (opperation)(i, &mut clusters, &mut *param_handle.lock().unwrap());
                    }
                }
            });
        }
    }

    pub fn stop(&mut self) {
        *self.run_handle.lock().unwrap() = false;
    }
}