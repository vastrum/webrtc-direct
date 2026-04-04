pub struct ConnectionLimiter {
    per_ip: HashMap<IpAddr, Arc<AtomicUsize>>,
    total: Arc<AtomicUsize>,
}

impl ConnectionLimiter {
    pub fn new() -> Self {
        Self { per_ip: HashMap::new(), total: Arc::new(AtomicUsize::new(0)) }
    }

    pub fn allow(&mut self, ip: IpAddr) -> Option<ConnGuard> {
        if self.total.load(Ordering::Relaxed) >= MAX_WEBRTC_CONNECTIONS {
            return None;
        }
        let ip_count = self.per_ip.entry(ip).or_default();
        if ip_count.load(Ordering::Relaxed) >= MAX_CONNS_PER_IP {
            return None;
        }
        ip_count.fetch_add(1, Ordering::Relaxed);
        self.total.fetch_add(1, Ordering::Relaxed);
        Some(ConnGuard { total: self.total.clone(), per_ip: ip_count.clone() })
    }
}

pub struct ConnGuard {
    total: Arc<AtomicUsize>,
    per_ip: Arc<AtomicUsize>,
}

impl Drop for ConnGuard {
    fn drop(&mut self) {
        self.per_ip.fetch_sub(1, Ordering::Relaxed);
        self.total.fetch_sub(1, Ordering::Relaxed);
    }
}

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use webrtc_direct_protocol::limits::{MAX_CONNS_PER_IP, MAX_WEBRTC_CONNECTIONS};
