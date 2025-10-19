use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use tokio::sync::Mutex;

pub struct GrpcClientPool<T> {
    clients: Vec<Arc<Mutex<T>>>,
    next_index: AtomicUsize,
}

impl<T> GrpcClientPool<T> {
    pub fn new(clients: Vec<T>) -> Self {
        let clients = clients
            .into_iter()
            .map(|c| Arc::new(Mutex::new(c)))
            .collect();

        Self {
            clients,
            next_index: AtomicUsize::new(0),
        }
    }

    pub fn get_client(&self) -> Arc<Mutex<T>> {
        let index = self.next_index.fetch_add(1, Ordering::Relaxed);
        self.clients[index % self.clients.len()].clone()
    }

    pub fn size(&self) -> usize {
        self.clients.len()
    }

    pub fn get_client_by_shard(&self, shard_index: usize) -> Arc<Mutex<T>> {
        self.clients[shard_index % self.clients.len()].clone()
    }
}

pub fn get_shard_for_user(user_id: &str, pool_size: usize) -> usize {
    let mut hasher = DefaultHasher::new();
    user_id.hash(&mut hasher);
    (hasher.finish() as usize) % pool_size
}

impl<T> Clone for GrpcClientPool<T> {
    fn clone(&self) -> Self {
        Self {
            clients: self.clients.clone(),
            next_index: AtomicUsize::new(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_round_robin() {
        let clients = vec![1, 2, 3, 4, 5];
        let pool = GrpcClientPool::new(clients);

        assert_eq!(pool.size(), 5);

        for expected in 1..=5 {
            let client = pool.get_client();
            let client = client.blocking_lock();
            assert_eq!(*client, expected);
        }

        let client = pool.get_client();
        let client = client.blocking_lock();
        assert_eq!(*client, 1);
    }

    #[test]
    fn test_pool_clone() {
        let clients = vec![1, 2, 3];
        let pool = GrpcClientPool::new(clients);
        let pool_clone = pool.clone();

        assert_eq!(pool.size(), pool_clone.size());
    }
}
