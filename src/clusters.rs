use std::{
    sync::{Arc, Mutex}
};

use super::ThreadIndex;

pub type ClusterIterHandler<PoolItem, LocalData> = fn(&usize, &mut Vec<PoolItem>, &mut LocalData);


//#[derive(Clone)]
pub struct Cluster<PoolItem, SharedData, LocalData> 
    where   PoolItem: Default + Clone + Send,
            LocalData: Default,
{
    pub thread_id: ThreadIndex,
    
    pub(crate) pool: Vec<PoolItem>,
    pub(crate) pointer: usize,
    pub(crate) spawn_count: usize,
    pub(crate) spawns: Vec<usize>,
    pub(crate) free: Vec<usize>,

    pub(crate) shared_data: Arc<Mutex<SharedData>>,
    pub local_data: LocalData,
}

impl<PoolItem, SharedData, LocalData> Cluster<PoolItem, SharedData, LocalData> 
    where   PoolItem: Default + Clone + Send,
            LocalData: Default,
{
    pub fn new(id: usize, capacity: u32, shared_data: Arc<Mutex<SharedData>>) -> Self {
        let mut pool = Vec::with_capacity(capacity as usize);
        let mut free = Vec::with_capacity(capacity as usize);
        let spawns = Vec::with_capacity(capacity as usize);

        for i in 0..capacity { 
            pool.push(PoolItem::default()); 
            free.push(i as usize);
        }

        Cluster { 
            thread_id: ThreadIndex(id), 
            pool, pointer: 0, spawns, free, spawn_count: 0,
            shared_data, local_data: LocalData::default(),
         }
    }

    pub fn access_shared_data(&mut self, update_data_handler: fn(&mut SharedData)) {
        //let data_handle = Arc::clone(&self.shared_data);
        //{
            let data = &mut *self.shared_data.lock().unwrap();
            update_data_handler(data);
            //drop(data);
        //}
        //drop(data_handle);
    }

    pub fn fetch(&mut self, pointer: usize) -> &mut PoolItem {
        &mut self.pool[pointer]
    }

    pub fn spawn(&mut self) {
        if let Some(pointer) = &self.free.pop() {
            self.spawns.push(*pointer);
            self.spawn_count = self.spawns.len();
        }
    }

    pub fn destroy(&mut self, pointer: usize) {
        if let Some(index) = &self.spawns.iter().position(|x| *x == pointer) {
            self.free.push(self.spawns.remove(*index));
            self.spawn_count = self.spawns.len();
        }
    }

    pub fn iter_spawns(&mut self, action: ClusterIterHandler<PoolItem, LocalData>, local_data: &mut LocalData) {
        self.pointer = 0;

        while self.pointer < self.spawn_count {
            action(&self.spawns[self.pointer], &mut self.pool, local_data);
            self.pointer += 1;
        }
    } 
}


// Pool containing all clusters

pub struct ClusterPool<I, S, L> (Vec<Arc<Mutex<Cluster<I, S, L>>>>)
    where   I: Default + Clone + Send +'static,
            L: Default;

impl<I, S, L> ClusterPool<I, S, L>
    where   I: Default + Clone + Send +'static,
            L: Default,
{
    pub fn new(cluster_count: u8, cluster_size: u32, shared_data: &Arc<Mutex<S>>) -> Self {
        let mut clusters: Vec<Arc<Mutex<Cluster<I, S, L>>>> = Vec::with_capacity(cluster_count as usize);
        for i in 0..cluster_count { 
            clusters.push(
                Arc::new(
                    Mutex::new(
                        Cluster::new(
                            i as usize, 
                            cluster_size, 
                            Arc::clone(shared_data),
                        )
                    )
                )
            );
        }
        ClusterPool(clusters)
    }

    pub(crate) fn arc_clone_cluster(&self, on_thread: &ThreadIndex) -> Arc<Mutex<Cluster<I, S, L>>> {
        Arc::clone(&self.0[*on_thread.value()])
    }

    // pub fn access_cluster(&mut self, on_thread: &ThreadIndex, update_cluster_handler: fn(&mut Cluster<I, S, L>)) {
    //     let cluster_handle = Arc::clone(&self.0[*on_thread.value()]);
    //     {
    //         let mut cluster = cluster_handle.lock().unwrap();
    //         update_cluster_handler(&mut cluster);
    //         drop(cluster);
    //     }
    //     drop(cluster_handle);
    // }


    pub fn len(&self) -> usize { self.0.len() }
}

impl<I, S, L> Clone for ClusterPool<I, S, L>
    where   I: Default + Clone + Send +'static,
            L: Default,
{
    fn clone(&self) -> Self {
        let mut clusters: Vec<Arc<Mutex<Cluster<I, S, L>>>> = Vec::new();
        for i in 0..self.0.len() { 
            clusters.push(Arc::clone(&self.0[i]));
        }
        ClusterPool(clusters)
    } 
}