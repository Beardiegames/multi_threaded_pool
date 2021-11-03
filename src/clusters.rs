use std::{
    sync::{Arc, Mutex}
};

use super::ClusterIndex;


#[derive(Clone)]
pub struct Cluster<PoolItem: Default + Clone + Send> {
    pub id: usize,
    pub(crate) pool: Vec<PoolItem>,
    pub(crate) pointer: usize,
    pub(crate) spawn_count: usize,
    pub(crate) spawns: Vec<usize>,
    pub(crate) free: Vec<usize>,
}

impl<PoolItem: Default + Clone + Send> Cluster<PoolItem> {

    pub fn new(id: usize, capacity: u32) -> Self {
        let mut pool = Vec::with_capacity(capacity as usize);
        let mut free = Vec::with_capacity(capacity as usize);
        let spawns = Vec::with_capacity(capacity as usize);

        for i in 0..capacity { 
            pool.push(PoolItem::default()); 
            free.push(i as usize);
        }

        Cluster { id, pool, pointer: 0, spawns, free, spawn_count: 0 }
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

    pub fn iter_spawns<P>(&mut self, action: fn(&usize, &mut Vec<PoolItem>, &mut P), params: &mut P) {
        self.pointer = 0;

        while self.pointer < self.spawn_count {
            action(&self.spawns[self.pointer], &mut self.pool, params);
            self.pointer += 1;
        }
    } 
}

pub struct ClusterPool<PoolItem> (Vec<Arc<Mutex<Cluster<PoolItem>>>>)
    where   PoolItem: Default + Clone + Send +'static;

impl<PoolItem> ClusterPool<PoolItem>
    where   PoolItem: Default + Clone + Send +'static
{
    pub fn new(cluster_count: u8, cluster_size: u32) -> Self {
        let mut clusters: Vec<Arc<Mutex<Cluster<PoolItem>>>> = Vec::with_capacity(cluster_count as usize);
        for i in 0..cluster_count { 
            clusters.push(Arc::new(Mutex::new(Cluster::new(i as usize, cluster_size))));
        }
        ClusterPool(clusters)
    }

    pub fn arc_cluster(&self, target_cluster: ClusterIndex) -> Arc<Mutex<Cluster<PoolItem>>> {
        Arc::clone(&self.0[target_cluster])
    }

    pub fn len(&self) -> usize { self.0.len() }
}

impl<PoolItem> Clone for ClusterPool<PoolItem>
    where   PoolItem: Default + Clone + Send +'static
{
    fn clone(&self) -> Self {
        let mut clusters: Vec<Arc<Mutex<Cluster<PoolItem>>>> = Vec::new();
        for i in 0..self.0.len() { 
            clusters.push(Arc::clone(&self.0[i]));
        }
        ClusterPool(clusters)
    } 
}