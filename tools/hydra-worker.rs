extern crate hydra;

use hydra::hosts::{Hostfile};
use hydra::server::{ResourceCacheConfig};
use hydra::worker::{WorkerServer};

use std::env;
use std::path::{PathBuf};

fn main() {
  let args: Vec<_> = env::args().collect();
  let num_workers = 4;
  let hostfile = Hostfile::new(&PathBuf::from(&args[1]));
  let cache_cfg = ResourceCacheConfig{
    port_range:         6000 .. 7000,
    dev_idx_range:      0 .. 2,
    dev_idx_overprov:   2,
  };
  let mut worker = WorkerServer::new(num_workers, hostfile, cache_cfg);
  worker.runloop();
}
