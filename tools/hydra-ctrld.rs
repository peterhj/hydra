extern crate hydra;

use hydra::control::{ControlServer};
use hydra::hosts::{Hostfile};
//use hydra::server::{ResourceCacheConfig};
//use hydra::worker::{WorkerServer};

use std::env;
use std::path::{PathBuf};

fn main() {
  let args: Vec<_> = env::args().collect();
  let hostfile = Hostfile::new(&PathBuf::from(&args[1]));
  let mut ctrl_serv = ControlServer::new(hostfile);
  ctrl_serv.runloop();
}
