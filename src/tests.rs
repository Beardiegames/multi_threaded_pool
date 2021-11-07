#[cfg(test)]

use std::{
    time::Duration,
    sync::{Arc, Mutex}, 
    thread::{self}
};

use crate::ObjectPool;

#[allow(unused)]
use super::{ThreadIndex, ThreadPool, Cluster, Spawn};

#[derive(Default, Clone)]
struct PoolObject(bool);

#[allow(unused)]
struct Params {
    pub setup_called: bool,
    pub opperation_called: bool,
}

#[test]
fn start_stop_threads() {
    let mut thread_pool = ThreadPool::<PoolObject, _, bool>::new(10, 10, ());
    
    thread_pool.start(
        |_c|{}, 
        |_c, _dt|{}
    );
    thread::sleep(Duration::from_millis(10));
    assert!(*thread_pool.run_handle.lock().unwrap());
    
    thread_pool.stop();
    thread::sleep(Duration::from_millis(10));
    assert!(!*thread_pool.run_handle.lock().unwrap());

    thread_pool.start(
        |_c|{}, 
        |_c, _dt|{}
    );
    thread::sleep(Duration::from_millis(10));
    assert!(*thread_pool.run_handle.lock().unwrap());

    thread_pool.start(
        |_c|{}, 
        |_c, _dt|{}
    );
    thread::sleep(Duration::from_millis(10));
    assert!(*thread_pool.run_handle.lock().unwrap());

    thread_pool.stop();
    thread::sleep(Duration::from_millis(10));
    assert!(!*thread_pool.run_handle.lock().unwrap());
}

#[test]
fn run_thread_handlers_are_called() {
    
    let params = Params {
        setup_called: false,
        opperation_called: false,
    };

    let mut thread_pool = ThreadPool::<PoolObject, Params, bool>
        ::new(1, 2, params);

    assert_eq!((*thread_pool.shared_data.lock().unwrap()).setup_called, false);
    assert_eq!((*thread_pool.shared_data.lock().unwrap()).opperation_called, false);
    
    thread_pool.start(
        |c|{ 
            c.access_shared_data(|d| d.setup_called = true);
        }, 
        |c, _dt|{ 
            c.access_shared_data(|d| d.opperation_called = true);
        }
    );
    thread::sleep(Duration::from_millis(10));
    
    thread_pool.stop();
    thread::sleep(Duration::from_millis(10));
    assert_eq!((*thread_pool.shared_data.lock().unwrap()).setup_called, true);
    assert_eq!((*thread_pool.shared_data.lock().unwrap()).opperation_called, true);

}

#[test]
fn shared_and_local_data_can_be_accessed_within_an_opperation () {

    let mut thread_pool = ThreadPool::<PoolObject, usize, usize>::new(2, 5_000_000, 0);

    {
        let cluster_0_handle = thread_pool.clusters().arc_clone_cluster(&ThreadIndex(0));
        let cluster_1_handle = thread_pool.clusters().arc_clone_cluster(&ThreadIndex(1));        
        {
            let cluster_0 = &mut *cluster_0_handle.lock().unwrap();
            assert_eq!(cluster_0.count(), 0);
            drop(cluster_0);

            let cluster_1 = &mut *cluster_1_handle.lock().unwrap();  
            assert_eq!(cluster_1.count(), 0);
            drop(cluster_1);
        }
    }

    thread_pool.start(
        |_c|{}, 
        |_c, _dt|{ 
            _c.local_data += 1;
            _c.access_shared_data(|d| *d += 1);
        },
    );
    
    
    thread::sleep(Duration::from_millis(1000));
    thread_pool.stop();

    

    let total_updates: usize;
    let mut cluster_updates: (usize, usize) = (999,999);
    {
        let data_handle = &thread_pool.shared_data;
        total_updates = *data_handle.lock().unwrap();
        drop(data_handle);
        
    } 
    {
        let cluster_handle = thread_pool.clusters()
            .arc_clone_cluster(&ThreadIndex(0));
        let cluster = cluster_handle.lock().unwrap();
        cluster_updates.0 = cluster.local_data;
        drop(cluster);
        drop(cluster_handle);
    }
    //panic!("got here!");
    {
        let cluster_handle = thread_pool.clusters()
            .arc_clone_cluster(&ThreadIndex(1));
        let cluster = cluster_handle.lock().unwrap();
        cluster_updates.1 = cluster.local_data;
        drop(cluster);
        drop(cluster_handle);
    } 

    println!("total number of updated: thread-0 ({}) + thread-1 ({}) should be {} per second",
        cluster_updates.0, cluster_updates.1, total_updates
    );

    assert_eq!(total_updates, cluster_updates.0 + cluster_updates.1);
}

#[test]
fn a_thread_tracks_opperation_update_time() {
    let mut thread_pool = ThreadPool::<PoolObject, usize, (usize, f32)>::new(1, 5_000_000, 0);

    thread_pool.start(
        |_c|{
            thread::sleep(Duration::from_millis(10));
        }, 
        |_c, _dt|{ 
            _c.local_data.0 += 1;
            _c.local_data.1 += _dt;
            println!("num updates = {}, update time = {}, total time = {}", _c.local_data.0, _dt, _c.local_data.1);
            thread::sleep(Duration::from_millis(10));
        },
    );

    thread::sleep(Duration::from_millis(100));
    thread_pool.stop();

    let cluster_handle = thread_pool.clusters()
        .arc_clone_cluster(&ThreadIndex(0));
    let cluster = cluster_handle.lock().unwrap();
    
    println!("num updates = {}, total update time = {}, partial update time = {}",
        cluster.local_data.0,
        cluster.local_data.1,
        cluster.local_data.1 / cluster.local_data.0 as f32,
    );

    assert!(cluster.local_data.1 / cluster.local_data.0 as f32 >= 0.01);
    assert!(cluster.local_data.1 / cluster.local_data.0 as f32 <= 0.011);
}

#[test]
fn clusters_can_spawn_objects() {
    let mut cluster = Cluster::<bool, bool, bool>::new(0, 2, Arc::new(Mutex::new(false)));
    
    assert_eq!(cluster.count(), 0);

    let spawn_1 = cluster.spawn();
    assert_eq!(spawn_1, Some(Spawn{ id: 0, self_index: 0, pool_index: 0}));
    assert_eq!(cluster.count(), 1);

    let spawn_2 = cluster.spawn();
    assert_eq!(spawn_2, Some(Spawn{ id: 1, self_index: 1, pool_index: 1}));
    assert_eq!(cluster.count(), 2);

    let spawn_3 = cluster.spawn();
    assert_eq!(spawn_3, None);
    assert_eq!(cluster.count(), 2);
}

#[test]
fn clusters_can_destroy_objects() {
    let mut cluster = Cluster::<bool, bool, bool>::new(0, 2, Arc::new(Mutex::new(false)));
    
    let spawn_1 = cluster.spawn().unwrap();
    let spawn_2 = cluster.spawn().unwrap();
    assert_eq!(cluster.count(), 2);

    cluster.destroy(spawn_2);
    assert_eq!(cluster.count(), 1);

    cluster.destroy(spawn_1.clone());
    assert_eq!(cluster.count(), 0);

    let spawn_3 = cluster.spawn().unwrap();
    assert_ne!(spawn_1, spawn_3);
    assert_eq!(spawn_3, Spawn{ id:2, self_index:0, pool_index:0 });
}

#[test]
fn clusters_can_iter_over_objects() {
    let mut cluster = Cluster::<bool, bool, ()>::new(0, 2, Arc::new(Mutex::new(false)));
    
    let spawn_1 = cluster.spawn().unwrap();
    let spawn_2 = cluster.spawn().unwrap();
    
    assert_eq!(cluster.fetch(&spawn_1), Some(&mut false));
    assert_eq!(cluster.fetch(&spawn_2), Some(&mut false));

    cluster.iter(|pool, _params|{ *pool.target() = true; });

    assert_eq!(cluster.fetch(&spawn_1), Some(&mut true));
    assert_eq!(cluster.fetch(&spawn_2), Some(&mut true));
}

#[test]
fn clusters_have_factories_for_spawning_specific_types() {
    let mut cluster = Cluster::<bool, bool, bool>::new(0, 2, Arc::new(Mutex::new(false)));
    cluster.set_build_factory("t-factory", |x|{
        println!("factory called");
        *x = true;
    });
    
    let spawn_1 = cluster.spawn().unwrap();
    assert_eq!(cluster.fetch(&spawn_1), Some(&mut false));

    let spawn_2 = cluster.build("t-factory").unwrap();

    assert_eq!(cluster.fetch(&spawn_1), Some(&mut false));
    assert_eq!(cluster.fetch(&spawn_2), Some(&mut true));
}