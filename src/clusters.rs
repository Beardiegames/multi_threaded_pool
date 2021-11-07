use std::{
    sync::{Arc, Mutex}
};

use crate::ThreadIndex;

pub type ClusterIterHandler<ItemType, LocalData> = fn(&mut ObjectPool<ItemType>, &mut LocalData);

#[derive(Clone, Debug, PartialEq)]
pub struct Spawn {
    pub(crate) id: u128,
    pub(crate) self_index: usize,
    pub(crate) pool_index: usize,
}

pub struct Factory {
    self_index: usize,
    opperation_index: usize,
}

struct ItemRef {
    pool_index: usize,
    spawn_index: usize,
}

pub struct ObjectPool<ItemType> 
where   ItemType: Default + Clone + Send
{
    pub(crate) items: Vec<ItemType>,
    pub(crate) active_pool_count: usize,
    pub(crate) spawn_id_counter: u128,
    pub(crate) all_spawns: Vec<Spawn>,
    pub(crate) iter_position: usize,

    free_pool_items: Vec<ItemRef>,
    active_pool_items: Vec<ItemRef>,
}

impl<ItemType> ObjectPool<ItemType> 
where   ItemType: Default + Clone + Send
{
    pub fn new(id: usize, capacity: u32) -> Self {
        let mut items = Vec::with_capacity(capacity as usize);
        let mut free_pool_items = Vec::with_capacity(capacity as usize);
        let mut all_spawns = Vec::with_capacity(capacity as usize);
        let active_pool_items = Vec::with_capacity(capacity as usize);

        for i in 0..capacity { 
            items.push(ItemType::default()); 
            free_pool_items.push(ItemRef{ pool_index: (capacity - (i + 1)) as usize, spawn_index: 0 });
            all_spawns.push(Spawn{ id:0, self_index: i as usize, pool_index: 0 });
        }

        ObjectPool { 
            items, all_spawns,
            active_pool_count: 0, spawn_id_counter: 0, iter_position: 0,
            free_pool_items, active_pool_items,
         }
    }

    pub fn target(&mut self) -> &mut ItemType {
        &mut self.items[self.active_pool_items[self.iter_position].pool_index]
    }

    pub fn target_spawn(&self) -> &Spawn {
        &self.all_spawns[self.active_pool_items[self.iter_position].spawn_index]
    }

    pub fn fetch(&mut self, spawn: &Spawn) -> Option<&mut ItemType> {
        if self.all_spawns[spawn.self_index].id == spawn.id {
            Some (&mut self.items[spawn.pool_index])
        } else {
            None
        }
    }

    pub fn fetch_raw(&mut self, pool_index: usize) -> &mut ItemType {
        &mut self.items[pool_index]
    }

    pub fn spawn(&mut self) -> Option<Spawn> {
        match self.free_pool_items.pop() {
            Some(mut iref) => {
                self.all_spawns[self.active_pool_count].id = self.spawn_id_counter;
                self.all_spawns[self.active_pool_count].pool_index = iref.pool_index;
                iref.spawn_index = self.active_pool_count;

                self.spawn_id_counter += 1;
                self.active_pool_items.push(iref);
                self.active_pool_count = self.active_pool_items.len();
    
                Some(self.all_spawns[self.active_pool_count-1].clone())
            },
            _ => None,
        }
    }

    pub fn destroy(&mut self, spawn: Spawn) {
        if self.all_spawns[spawn.self_index].id == spawn.id {
            match self.active_pool_items.iter().position(|x| x.pool_index == spawn.pool_index){
                Some(active_index) => {
                    self.free_pool_items.push(self.active_pool_items.remove(active_index));
                    self.active_pool_count = self.active_pool_items.len();
                },
                _ => {},
            }
        }
    }

    pub fn capacity(&self) -> usize { self.items.len() }
    pub fn count(&self) -> usize { self.active_pool_count }
}

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
            pool: ObjectPool::new(id, capacity),
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