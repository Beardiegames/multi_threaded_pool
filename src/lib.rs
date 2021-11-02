use std::{
    sync::{Arc, Mutex}, 
    thread::{self}
};


pub type ThreadIndex = usize;
pub type ClusterIndex = usize;

pub type OpperationHandler<PoolItem, Parameters> = fn(ThreadIndex, &mut ClusterPool<PoolItem>, &mut Parameters);
pub type SetupHandler<PoolItem, Parameters> = fn(&mut Cluster<PoolItem>, &mut Parameters);


pub struct ClusterPool<PoolItem> (Vec<Arc<Mutex<Cluster<PoolItem>>>>)
    where   PoolItem: Default + Clone + Send +'static;

impl<PoolItem> ClusterPool<PoolItem>
    where   PoolItem: Default + Clone + Send +'static
{
    pub fn new(cluster_count: u8, cluster_size: u16) -> Self {
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

// ThreadPool

pub struct ThreadPool<PoolItem, Parameters>
    where   PoolItem: Default + Clone + Send +'static, 
            Parameters: Send + 'static
{
    pub cluster_capacity: u16,
    pub run_handle: Arc<Mutex<bool>>,
    pub clusters: ClusterPool<PoolItem>,
    pub parameters: Arc<Mutex<Parameters>>,
}

impl<PoolItem, Parameters> ThreadPool<PoolItem, Parameters>
    where   PoolItem: Default + Clone + Send +'static, 
            Parameters: Send + 'static
{
    pub fn new(cluster_count: u8, cluster_size: u16, parameters: Parameters) -> Self {
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


// Cluster

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

    pub fn new(id: usize, capacity: u16) -> Self {
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


// Tests

#[cfg(test)]
mod tests {
    use std::time::Duration;

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[derive(Default, Clone)]
    struct PoolObject(bool);

    struct Params {
        pub setup_called: bool,
        pub opperation_called: bool,
    }

    #[test]
    fn start_stop_threads() {
        let mut process = ThreadPool::<PoolObject, _>::new(10, 10, ());
        
        process.start(
            |_c, _p|{}, 
            |_t, _c, _p|{}
        );
        thread::sleep(Duration::from_millis(10));
        assert!(*process.run_handle.lock().unwrap());
        
        process.stop();
        thread::sleep(Duration::from_millis(10));
        assert!(!*process.run_handle.lock().unwrap());

        process.start(
            |_c, _p|{}, 
            |_t, _c, _p|{}
        );
        thread::sleep(Duration::from_millis(10));
        assert!(*process.run_handle.lock().unwrap());

        process.start(
            |_c, _p|{}, 
            |_t, _c, _p|{}
        );
        thread::sleep(Duration::from_millis(10));
        assert!(*process.run_handle.lock().unwrap());

        process.stop();
        thread::sleep(Duration::from_millis(10));
        assert!(!*process.run_handle.lock().unwrap());
    }

    #[test]
    fn running_thread_call_handlers() {
        
        let params = Arc::new(Mutex::new(Params {
            setup_called: false,
            opperation_called: false,
        }));

        let mut process = ThreadPool::<PoolObject, Arc<Mutex<Params>>>
            ::new(1, 2, Arc::clone(&params));

        assert_eq!((*params.lock().unwrap()).setup_called, false);
        assert_eq!((*params.lock().unwrap()).opperation_called, false);
        
        process.start(
            |_c, p|{ (*p.lock().unwrap()).setup_called = true; }, 
            |_t, _c, p|{ (*p.lock().unwrap()).opperation_called = true; },
        );
        thread::sleep(Duration::from_millis(10));
        
        process.stop();
        thread::sleep(Duration::from_millis(10));
        assert_eq!((*params.lock().unwrap()).setup_called, true);
        assert_eq!((*params.lock().unwrap()).opperation_called, true);

    }

    #[test]
    fn a_thread_can_access_all_clusters () {

        let paramaters: (usize, usize) = (0,0);
        let mut process = ThreadPool::<PoolObject, _>::new(2, 64_000, paramaters);

        {
            assert_eq!(process.clusters().arc_cluster(0).lock().unwrap().spawn_count, 0);
            assert_eq!(process.clusters().arc_cluster(1).lock().unwrap().spawn_count, 0);
        }
        process.start(
            |_c, _p|{}, 
            |_t, _c, _p|{ 
                if _t ==0 {
                    _p.0 += 1;
                    drop(_p);
                    let cluster_0 = _c.arc_cluster(0);
                    cluster_0.lock().unwrap().spawn();
                } 
                else if _t ==1 {
                    _p.1 += 1;
                    drop(_p);
                    {
                        let cluster_0 = _c.arc_cluster(0);
                        cluster_0.lock().unwrap().spawn();
                    }
                    let cluster_1 = _c.arc_cluster(1);
                    cluster_1.lock().unwrap().spawn();
                }
            },
        );
        
        
        thread::sleep(Duration::from_millis(10));
        process.stop();

        let mut num_updates = (666,666);
        let mut spawn_count = (999,999);
        {
            let param_handle = &process.parameters();
            num_updates.0 = param_handle.lock().unwrap().0;
            num_updates.1 = param_handle.lock().unwrap().1;
        } {
            let cluster_0_handle = &process.clusters().arc_cluster(0);
            spawn_count.0 = cluster_0_handle.lock().unwrap().spawn_count.clone();
        } {
            let cluster_1_handle = &process.clusters().arc_cluster(1);
            spawn_count.1 = cluster_1_handle.lock().unwrap().spawn_count.clone();
        } 

        assert_eq!(spawn_count.0, num_updates.0 + num_updates.1);
        assert_eq!(spawn_count.1, num_updates.1);
    }

    #[test]
    fn clusters_can_spawn_objects() {
        let mut cluster = Cluster::<bool>::new(0, 2);
        
        assert_eq!(cluster.spawn_count, 0);
        cluster.spawn();
        assert_eq!(cluster.spawn_count, 1);
        cluster.spawn();
        assert_eq!(cluster.spawn_count, 2);
        cluster.spawn();
        assert_eq!(cluster.spawn_count, 2);
    }

    #[test]
    fn clusters_can_destroy_objects() {
        let mut cluster = Cluster::<bool>::new(0, 2);
        
        cluster.spawn();
        cluster.spawn();
        assert_eq!(cluster.spawn_count, 2);
        cluster.destroy(0);
        assert_eq!(cluster.spawn_count, 1);
        cluster.destroy(1);
        assert_eq!(cluster.spawn_count, 0);
        cluster.destroy(0);
        assert_eq!(cluster.spawn_count, 0);
    }

    #[test]
    fn clusters_can_iter_over_objects() {
        let mut cluster = Cluster::<bool>::new(0, 2);
        
        cluster.spawn();
        cluster.spawn();
        
        assert_eq!(*cluster.fetch(0), false);
        assert_eq!(*cluster.fetch(1), false);

        cluster.iter_spawns(
            |target, pool, _params|{
                pool[*target] = true;
            },
            &mut (),
        );

        assert_eq!(*cluster.fetch(0), true);
        assert_eq!(*cluster.fetch(1), true);
    }
}