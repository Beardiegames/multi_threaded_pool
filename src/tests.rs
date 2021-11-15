#[cfg(test)]

#[warn(unused_imports)]
use std::{
    time::Duration,
    sync::{Arc, Mutex}, 
    thread::{self}
};

#[warn(unused_imports)]
use crate::{DataCell, ObjectPool, DataManager};

#[allow(unused)]
use super::{ThreadPool, Cluster, Spawn};

#[derive(Default, Clone)]
struct PoolObject(bool);

#[allow(unused)]
#[derive(Default, Clone, Debug)]
struct Params {
    pub setup_called: bool,
    pub opperation_called: bool,
}

#[test]
fn start_stop_threads() {
    let mut thread_pool = ThreadPool::<PoolObject, bool>::new(10, 10);
    
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
    
    let mut thread_pool = ThreadPool::<PoolObject, Params>::new(1, 2);

    assert_eq!(thread_pool.shared.unlinked(0).setup_called, false);
    assert_eq!(thread_pool.shared.unlinked(0).opperation_called, false);
    
    thread_pool.start(
        |c|{ 
            c.shared.write(0, |d| d.setup_called = true);
        }, 
        |c, _dt|{ 
            c.shared.write(0, |d| d.opperation_called = true);
        }
    );
    thread::sleep(Duration::from_millis(1));
    
    thread_pool.stop();
    thread::sleep(Duration::from_millis(10));
    assert_eq!(thread_pool.shared.unlinked(0).setup_called, true);
    assert_eq!(thread_pool.shared.unlinked(0).opperation_called, true);
}

#[test]
fn local_data_can_be_accessed_on_all_threads () {

    let mut thread_pool = ThreadPool::<PoolObject, (usize, usize)>::new(2, 5_000_000);

    thread_pool.start(
        |_c|{}, 
        |_c, _dt|{ 
            _c.shared.write(_c.thread_id,|d| d.0 += 1);
            _c.shared.write(0,|d| d.1 += 1);
        },
    );
    
    
    thread::sleep(Duration::from_millis(1000));
    thread_pool.stop();

    

    let total_updates = thread_pool.shared.unlinked(0).1;
    let mut cluster_updates: (usize, usize) = (
        thread_pool.shared.unlinked(0).0,
        thread_pool.shared.unlinked(1).0
    );

    println!("total number of updated: thread-0 ({}) + thread-1 ({}) should be {} per second",
        cluster_updates.0, cluster_updates.1, total_updates
    );

    assert_eq!(total_updates, cluster_updates.0 + cluster_updates.1);
}

#[test]
fn a_thread_tracks_opperation_update_time() {
    let mut thread_pool = ThreadPool::<PoolObject, (usize, f32)>::new(1, 5_000_000);

    thread_pool.start(
        |_c|{
            println!("setup called");
            thread::sleep(Duration::from_millis(10));
        }, 
        |_c, _dt| {    
            println!("opperation called, with dt = {}", _dt);

            _c.shared.catch(_c.thread_id, 
                _dt, 
                |v, d| {
                    d.0 += 1;
                    d.1 = d.1 + *v;
                }
            );
            let unlinked_data = _c.shared.unlinked(_c.thread_id);
            println!("thread: {}, num updates = {}, update time = {}, total time = {}",
                _c.thread_id,
                unlinked_data.0,
                _dt,
                unlinked_data.1
            );
            thread::sleep(Duration::from_millis(10));
        },
    );

    thread::sleep(Duration::from_millis(1000));
    thread_pool.stop();


    let unlinked_data = thread_pool.shared.unlinked(0);
    let dt_millis = (unlinked_data.1 * 1000.0).ceil() as usize / unlinked_data.0;
    
    assert!(unlinked_data.0 > 0);

    println!("num updates = {}, total update time = {}, partial update time = {}",
        unlinked_data.0,
        unlinked_data.1,
        dt_millis,
    );

    assert!(dt_millis >= 10);
    assert!(dt_millis <= 11);
}

#[test]
fn clusters_can_spawn_objects() {
    let mut cluster = Cluster::<bool, bool>::new(0, 2, DataManager::new(1));
    
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
    let mut cluster = Cluster::<bool, bool>::new(0, 2, DataManager::new(1));
    
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
    let mut cluster = Cluster::<bool, bool>::new(0, 2, DataManager::new(1));
    
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
    let mut cluster = Cluster::<bool, bool>::new(0, 2, DataManager::new(1));
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