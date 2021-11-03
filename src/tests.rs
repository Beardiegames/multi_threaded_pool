use std::{
    time::Duration,
    sync::{Arc, Mutex}, 
    thread::{self}
};

use super::{ThreadPool, Cluster};


#[cfg(test)]

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
    let mut process = ThreadPool::<PoolObject, _>::new(2, 5_000_000, paramaters);

    {
        assert_eq!(process.clusters().arc_cluster(0).lock().unwrap().spawn_count, 0);
        assert_eq!(process.clusters().arc_cluster(1).lock().unwrap().spawn_count, 0);
    }
    process.start(
        |_c, _p|{}, 
        |_t, _c, _p|{ 
            if _t ==0 {
                _p.0 += 1;
                let cluster_0 = _c.arc_cluster(0);
                cluster_0.lock().unwrap().spawn();
                drop(_p);
            } 
            else if _t ==1 {
                _p.1 += 1;
                {
                    let cluster_0 = _c.arc_cluster(0);
                    cluster_0.lock().unwrap().spawn();
                }
                let cluster_1 = _c.arc_cluster(1);
                cluster_1.lock().unwrap().spawn();
                drop(_p);
            }
        },
    );
    
    
    thread::sleep(Duration::from_millis(1000));
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

    panic!("total number of updated: thread-0 ({}) + thread-1 ({}) = {} per second",
        num_updates.0, num_updates.1, num_updates.0 + num_updates.1
    )
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