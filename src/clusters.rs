use std::{
    sync::{Arc, Mutex}
};

use crate::{Spawn, ThreadIndex, pooling::ObjectPool};

pub type ClusterIterHandler<ItemType, LocalData> = fn(&mut ObjectPool<ItemType>, &mut LocalData);

pub struct Cluster<ItemType, SharedData, LocalData> 
where   ItemType: Default + Clone + Send,
        LocalData: Default,
{
    pub thread_id: ThreadIndex,  

    pub(crate) pool: ObjectPool<ItemType>,
    pub(crate) factories: Vec<(&'static str, fn(&mut ItemType))>,

    pub(crate) shared_data: Arc<Mutex<SharedData>>,
    pub local_data: LocalData,
}

impl<ItemType, SharedData, LocalData> Cluster<ItemType, SharedData, LocalData> 
where   ItemType: Default + Clone + Send,
        LocalData: Default,
{
    pub fn new(id: usize, capacity: u32, shared_data: Arc<Mutex<SharedData>>) -> Self {
        Cluster { 
            thread_id: ThreadIndex(id), 
            pool: ObjectPool::new(capacity),
            factories: Vec::new(),
            shared_data, local_data: LocalData::default(),
         }
    }

    pub fn access_shared_data(&mut self, update_data_handler: fn(&mut SharedData)) {

        let data = &mut *self.shared_data.lock().unwrap();
        update_data_handler(data);
    }

    pub fn set_build_factory(&mut self, tag: &'static str, factory_callback: fn(&mut ItemType)) {
        self.factories.push((tag, factory_callback));
    }

    pub fn build(&mut self, tag: &'static str) -> Option<Spawn> {
        match &self.factories.iter().position(|x| x.0 == tag) {
            Some(f_index) => {
                if let Some(spawn) = self.pool.spawn() {
                    (self.factories[*f_index].1)(&mut self.pool.items[spawn.pool_index]);
                    Some(spawn)
                } else {
                    None
                }
            },
            _ => None,
        }
    }

    pub fn fetch(&mut self, spawn: &Spawn) -> Option<&mut ItemType> {
        self.pool.fetch(spawn)
    }

    pub fn fetch_raw(&mut self, pool_index: usize) -> &mut ItemType {
        self.pool.fetch_raw(pool_index)
    }

    pub fn spawn(&mut self) -> Option<Spawn> {
        self.pool.spawn()
    }

    pub fn destroy(&mut self, spawn: Spawn) {
        self.pool.destroy(spawn)
    }

    pub fn iter(&mut self, handler: ClusterIterHandler<ItemType, LocalData>, local_data: &mut LocalData) {
        self.pool.iter_position = 0;

        while &self.pool.iter_position < &self.pool.active_pool_count {
            handler(&mut self.pool, local_data);
            self.pool.iter_position += 1;
        }
    }

    pub fn capacity(&self) -> usize { self.pool.items.len() }
    pub fn count(&self) -> usize { self.pool.active_pool_count }
}


// ObjectPool containing all clusters

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