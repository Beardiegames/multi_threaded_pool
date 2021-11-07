use std::{sync::{Arc, Mutex}, thread::{self}, time::{SystemTime}};

mod tests;
mod pooling;
mod clusters;

pub use pooling::{ Spawn, ObjectPool };
pub use clusters::{ ClusterPool, Cluster };

pub struct ThreadIndex(usize);

impl ThreadIndex {
    pub fn value(&self) -> &usize { &self.0 }
}

pub type ThreadSetupHandler<PoolItem, SharedData, LocalData> = fn(
    &mut Cluster<PoolItem, SharedData, LocalData>,
);
pub type ThreadUpdateHandler<PoolItem, SharedData, LocalData> = fn(
    &mut Cluster<PoolItem, SharedData, LocalData>,
    &f32,
);

pub struct ThreadPool<PoolItem, SharedData, LocalData>
    where   PoolItem: Default + Clone + Send +'static, 
            SharedData: Send + 'static,
            LocalData: Default + Send + 'static,
{
    pub cluster_capacity: u32,
    pub run_handle: Arc<Mutex<bool>>,
    pub clusters: ClusterPool<PoolItem, SharedData, LocalData>,
    pub shared_data: Arc<Mutex<SharedData>>,
}

impl<PoolItem, SharedData, LocalData> ThreadPool<PoolItem, SharedData, LocalData>
    where   PoolItem: Default + Clone + Send +'static, 
            SharedData: Send + 'static,
            LocalData: Default + Send + 'static,
{
    pub fn new(cluster_count: u8, cluster_size: u32, shared_data: SharedData) -> Self {
        let shared_data = Arc::new(Mutex::new(shared_data));

        ThreadPool { 
            cluster_capacity: cluster_size,
            run_handle: Arc::new(Mutex::new(false)),
            clusters: ClusterPool::new(cluster_count, cluster_size, &shared_data),
            shared_data,
        }
    }

    pub fn clusters(&mut self) -> &mut ClusterPool<PoolItem, SharedData, LocalData> {
        &mut self.clusters
    }

    pub fn start (
        &mut self, 
        setup: ThreadSetupHandler<PoolItem, SharedData, LocalData>, 
        opperation: ThreadUpdateHandler<PoolItem, SharedData, LocalData>,
    ) 
        where   PoolItem: Default + Clone + Send +'static,
                SharedData: Send + 'static,
                LocalData: Default + Send + 'static,
    {
        {
            let handle = Arc::clone(&self.run_handle);
            let mut running = handle.lock().unwrap();

            if *running { return; } 
            *running = true;
        }

        for i in 0..self.clusters.len() {
            let run_handle = Arc::clone(&self.run_handle);
            let thread_id = ThreadIndex(i);
            let cluster_handle = self.clusters.arc_clone_cluster(&thread_id);
            
            thread::spawn(move || {
                let mut play_time = SystemTime::now();
                let mut delta_time;
                {
                    let mut s_cluster = cluster_handle.lock().unwrap();
                    (setup)(&mut s_cluster);
                }
                {
                    let mut u_cluster = cluster_handle.lock().unwrap();
                    
                    'active: loop {
                        delta_time = play_time.elapsed().unwrap().as_millis() as f32 * 0.001;
                        play_time = SystemTime::now();
                        {
                            if !*run_handle.lock().unwrap() { break 'active; }
                        } {
                            (opperation)(&mut u_cluster, &delta_time);
                        }
                    }
                }
            });
        }
    }

    pub fn stop(&mut self) {
        *self.run_handle.lock().unwrap() = false;
    }
}