use std::{
    time::Duration,
    sync::{Arc, Mutex}, 
    thread::{self}
};

use super::{ThreadIndex, ThreadPool, Cluster};


#[cfg(test)]

#[derive(Default, Clone)]
struct PoolObject(bool);

struct Params {
    pub setup_called: bool,
    pub opperation_called: bool,
}

#[test]
fn start_stop_threads() {
    let mut thread_pool = ThreadPool::<PoolObject, _, bool>::new(10, 10, ());
    
    thread_pool.start(
        |_c|{}, 
        |_c|{}
    );
    thread::sleep(Duration::from_millis(10));
    assert!(*thread_pool.run_handle.lock().unwrap());
    
    thread_pool.stop();
    thread::sleep(Duration::from_millis(10));
    assert!(!*thread_pool.run_handle.lock().unwrap());

    thread_pool.start(
        |_c|{}, 
        |_c|{}
    );
    thread::sleep(Duration::from_millis(10));
    assert!(*thread_pool.run_handle.lock().unwrap());

    thread_pool.start(
        |_c|{}, 
        |_c|{}
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
        |c|{ 
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
fn a_cluster_can_access_shared_and_local_data () {

    let mut thread_pool = ThreadPool::<PoolObject, usize, usize>::new(2, 5_000_000, 0);

    {
        let cluster_0_handle = thread_pool.clusters().arc_clone_cluster(&ThreadIndex(0));
        let cluster_1_handle = thread_pool.clusters().arc_clone_cluster(&ThreadIndex(1));        
        {
            let cluster_0 = &mut *cluster_0_handle.lock().unwrap();
            assert_eq!(cluster_0.spawn_count, 0);
            drop(cluster_0);

            let cluster_1 = &mut *cluster_1_handle.lock().unwrap();  
            assert_eq!(cluster_1.spawn_count, 0);
            drop(cluster_1);
        }
    }

    thread_pool.start(
        |_c|{}, 
        |_c|{ 
            _c.local_data += 1;
            _c.access_shared_data(|d| *d += 1);
        },
    );
    
    
    thread::sleep(Duration::from_millis(1000));
    thread_pool.stop();

    

    let mut total_updates: usize = 666;
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
fn clusters_can_spawn_objects() {
    let mut cluster = Cluster::<bool, bool, bool>::new(0, 2, Arc::new(Mutex::new(false)));
    
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
    let mut cluster = Cluster::<bool, bool, bool>::new(0, 2, Arc::new(Mutex::new(false)));
    
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
    let mut cluster = Cluster::<bool, bool, _>::new(0, 2, Arc::new(Mutex::new(false)));
    
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