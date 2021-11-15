use std::{fmt::Debug, sync::Arc};

use crate::{Spawn, pooling::ObjectPool, shared::DataManager};

pub type ClusterIterHandler<ItemType, LocalData> = fn(&mut ObjectPool<ItemType>, &mut DataManager<LocalData>);

impl<LocalData: Default + Clone + Debug>  Clone for DataManager<LocalData> {
    fn clone(&self) -> Self {
        let mut cloned_data = Vec::with_capacity(self.data.len());
        for i in 0..self.data.len() { 
            cloned_data.push(Arc::clone(&self.data[i]));
        }
        DataManager{ data: cloned_data }
    }
}


pub struct Cluster<ItemType, LocalData> 
where   ItemType: Default + Clone + Send,
        LocalData: Default + Clone + Debug,
{
    pub(crate) thread_id: usize, //ThreadIndex,  

    pub(crate) pool: ObjectPool<ItemType>,
    pub(crate) factories: Vec<(&'static str, fn(&mut ItemType))>,

    pub shared: DataManager<LocalData>,
    //pub local_data: LocalData,
}

impl<ItemType, LocalData> Cluster<ItemType, LocalData> 
where   ItemType: Default + Clone + Send,
        LocalData: Default + Clone + Debug,
{
    pub fn new(id: usize, capacity: u32, shared_data_clone: DataManager<LocalData>) -> Self {

        Cluster { 
            thread_id: id, //ThreadIndex(id), 
            pool: ObjectPool::new(capacity),
            factories: Vec::new(),
            shared: shared_data_clone,
            //local_data: LocalData::default(),
         }
    }

    pub fn thread_id(&self) -> &usize { &self.thread_id }

    // pub fn shared_write(&mut self, thread_id: usize, access_handler: fn(&mut LocalData)) {

    //     let data = &mut *self.shared_data[thread_id].lock().unwrap();
    //     print!("pre write: {:?}" , data.0);
    //     access_handler(&mut data.0);
    //     println!("post write: {:?}" , data.0);
    // }

    // pub fn shared_clone(&self, thread_id: usize) -> LocalData {

    //     let handle = self.shared_data[thread_id].lock().unwrap();
    //     let clone = handle.0.clone();
    //     drop(handle);
    //     clone
    // }

    // pub fn local_write(&mut self, access_handler: fn(&mut LocalData)) {

    //     let data = &mut *self.shared_data[self.thread_id].lock().unwrap();
    //     print!("pre write: {:?}" , data.0);
    //     access_handler(&mut data.0);
    //     println!("post write: {:?}" , data.0);
    // }

    // pub fn local_clone(&self) -> LocalData {

    //     let handle = self.shared_data[self.thread_id].lock().unwrap();
    //     let clone = handle.0.clone();
    //     drop(handle);
    //     clone
    // }

    // pub(crate) fn shared_update(&self) {

    //     let mut handle = self.shared_data[self.thread_id].lock().unwrap();
    //     handle.0 = self.local_data.clone();
    //     println!("shared update: {:?}" , handle.0);
    //     //drop(handle);
    // }

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

    pub fn iter(&mut self, handler: ClusterIterHandler<ItemType, LocalData>) {
        self.pool.iter_position = 0;

        while &self.pool.iter_position < &self.pool.active_pool_count {
            handler(&mut self.pool, &mut self.shared);
            self.pool.iter_position += 1;
        }
    }

    pub fn capacity(&self) -> usize { self.pool.items.len() }
    pub fn count(&self) -> usize { self.pool.active_pool_count }
}


// ObjectPool containing all clusters

// pub struct ClusterPool<I, L> (Vec<Cluster<I, L>>) //(Vec<Arc<Mutex<Cluster<I, L>>>>)
// where   I: Default + Clone + Send +'static,
//         L: Default + Clone;

// impl<I, L> ClusterPool<I, L>
// where   I: Default + Clone + Send +'static,
//         L: Default + Clone,
// {
//     pub fn new(cluster_count: u8, cluster_size: u32, shared_data: &Vec<Arc<Mutex<L>>>) -> Self {
//         let mut clusters: Vec<Arc<Mutex<Cluster<I, L>>>> = Vec::with_capacity(cluster_count as usize);

//         for i in 0..cluster_count { 

//             let mut data_clone: Vec<Arc<Mutex<L>>> = Vec::new();
//             for i in 0..shared_data.len() { 
//                 data_clone.push(shared_data[i].clone());
//             }

//             clusters.push(
//                 Arc::new(
//                     Mutex::new(
//                         Cluster::new(
//                             i as usize, 
//                             cluster_size, 
//                             data_clone,
//                         )
//                     )
//                 )
//             );
//         }
//         ClusterPool(clusters)
//     }

//     // pub(crate) fn arc_clone_cluster(&self, on_thread: &usize) -> Arc<Mutex<Cluster<I, L>>> {
//     //     Arc::clone(&self.0[*on_thread])
//     // }

//     pub fn len(&self) -> usize { self.0.len() }
// }

// impl<I, L> Clone for ClusterPool<I, L>
// where   I: Default + Clone + Send +'static,
//         L: Default + Clone,
// {
//     fn clone(&self) -> Self {
//         let mut clusters: Vec<Arc<Mutex<Cluster<I, L>>>> = Vec::new();
//         for i in 0..self.0.len() { 
//             clusters.push(Arc::clone(&self.0[i]));
//         }
//         ClusterPool(clusters)
//     } 
// }