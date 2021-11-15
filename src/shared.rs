use std::sync::{Arc, Mutex};
use std::fmt::Debug;

use crate::DataCell;


pub struct DataManager<LocalData: Default + Clone + Debug> {
    pub(crate) data: Vec<Arc<Mutex<DataCell<LocalData>>>>,
}

impl<LocalData: Default + Clone + Debug> DataManager<LocalData> {
    pub fn new(cluster_count: u8) -> Self {
        let mut data = Vec::with_capacity(cluster_count as usize);
        for _i in 0..cluster_count as usize { 
            data.push(Arc::new(Mutex::new(DataCell::default())));
        }
        DataManager { data }
    }

    pub fn write(&mut self, thread_id: usize, data_handler: fn(&mut LocalData)) {

        let handle = &mut *self.data[thread_id].lock().unwrap();
        data_handler(&mut handle.0);
        drop(handle);
    }

    pub fn catch<T>(&mut self, thread_id: usize, value: &T, data_handler: fn(&T, &mut LocalData)) {

        let handle = &mut *self.data[thread_id].lock().unwrap();
        data_handler(value, &mut handle.0);
        drop(handle);
    }

    pub fn catch_mut<T>(&mut self, thread_id: usize, value: &mut T, data_handler: fn(&mut T, &mut LocalData)) {

        let handle = &mut *self.data[thread_id].lock().unwrap();
        data_handler(value, &mut handle.0);
        drop(handle);
    }

    pub fn unlinked(&self, thread_id: usize) -> LocalData {

        let handle = self.data[thread_id].lock().unwrap();
        let clone = handle.0.clone();
        drop(handle);
        clone
    }
}