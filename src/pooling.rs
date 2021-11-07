
#[derive(Clone, Debug, PartialEq)]
pub struct Spawn {
    pub(crate) id: u128,
    pub(crate) self_index: usize,
    pub(crate) pool_index: usize,
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
    pub fn new(capacity: u32) -> Self {
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