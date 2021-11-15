use std::{fmt::Debug, marker::PhantomData, sync::{Arc, Mutex}, thread::{self}, time::{SystemTime}};

mod tests;
mod pooling;
mod clusters;
mod shared;

use shared::DataManager;
pub use pooling::{ Spawn, ObjectPool };
pub use clusters::{ Cluster };

// pub struct ThreadIndex(usize);

// impl ThreadIndex {
//     pub fn value(&self) -> &usize { &self.0 }
// }
// impl PartialEq for ThreadIndex {
//     fn eq(&self, other: &ThreadIndex) -> bool {
//         self.0 == other.0
//     }
// }

pub type ThreadSetupHandler<PoolItem, LocalData> = fn(
    &mut Cluster<PoolItem, LocalData>,
);
pub type ThreadUpdateHandler<PoolItem, LocalData> = fn(
    &mut Cluster<PoolItem, LocalData>,
    &f32,
);

#[derive(Default)]
pub struct DataCell<LocalData>(LocalData);

pub struct ThreadPool<PoolItem, LocalData>
    where   PoolItem: Default + Clone + Send +'static, 
            LocalData: Default + Clone + Debug + Send + 'static,
{
    pub cluster_capacity: u32,
    pub run_handle: Arc<Mutex<bool>>,
    pub cluster_count: u8,
    //pub clusters: ClusterPool<PoolItem, LocalData>,
    pub shared: DataManager<LocalData>,
    pub phantom_data: PhantomData<PoolItem>,
}

impl<PoolItem, LocalData> ThreadPool<PoolItem, LocalData>
    where   PoolItem: Default + Clone + Send + 'static, 
            LocalData: Default + Clone + Debug + Send + 'static,
{
    pub fn new(cluster_count: u8, cluster_size: u32) -> Self {
        ThreadPool { 
            cluster_capacity: cluster_size,
            run_handle: Arc::new(Mutex::new(false)),
            cluster_count,
            //clusters: ClusterPool::new(cluster_count, cluster_size, &shared_data),
            shared: DataManager::new(cluster_count),
            phantom_data: PhantomData,
        }
    }

    pub fn start (
        &mut self, 
        setup: ThreadSetupHandler<PoolItem, LocalData>, 
        opperation: ThreadUpdateHandler<PoolItem, LocalData>,
    ) 
        where   PoolItem: Default + Clone + Send +'static,
                LocalData: Default + Send + 'static,
    {
        {
            let handle = Arc::clone(&self.run_handle);
            let mut running = handle.lock().unwrap();

            if *running { return; } 
            *running = true;
        }

        for i in 0..self.cluster_count as usize {
            let run_handle = Arc::clone(&self.run_handle);
            let thread_id = i;
            let capacity = self.cluster_capacity;
            let data_clone = self.shared.clone();
            
            thread::spawn(move || {
                let mut cluster = Cluster::new(
                    thread_id, capacity, data_clone
                );
                let mut play_time = SystemTime::now();
                let mut delta_time;
                {
                    //let mut s_cluster = cluster_handle.lock().unwrap();
                    (setup)(&mut cluster);
                }
                {
                    //let mut u_cluster = cluster_handle.lock().unwrap();
                    
                    'active: loop {
                        delta_time = play_time.elapsed().unwrap().as_millis() as f32 * 0.001;
                        play_time = SystemTime::now();
                        {
                            if !*run_handle.lock().unwrap() { break 'active; }
                        } {
                            (opperation)(&mut cluster, &delta_time);
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