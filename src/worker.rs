use api::{Experiment};
use static_hosts::{CONTROL_BCAST_ADDR, CONTROL_SOURCE_ADDR, CONTROL_SINK_ADDR};
use server::{Trial, ResourceCacheConfig, ResourceCache, ProtocolMsg};

//use comm::spmc;
use chan;
use nanomsg;
use rustc_serialize::json;
use threadpool::{ThreadPool};
//use zmq;

use std::fs::{File, create_dir};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::str::{from_utf8};
use std::sync::{Arc, Mutex};
//use std::sync::mpsc::{Sender, channel};
use std::thread::{sleep_ms};

enum State {
  Idle,
}

pub struct WorkerServer {
  //zmq_ctx:      zmq::Context,
  res_cache:    Arc<Mutex<ResourceCache>>,
  //work_tx:      Sender<(usize, Experiment)>,
  //work_tx:      spmc::unbounded::Producer<'a, (usize, Experiment)>,
  work_tx:      chan::Sender<(usize, Experiment)>,
  work_pool:    ThreadPool,
}

impl WorkerServer {
  pub fn new(cache_cfg: ResourceCacheConfig) -> WorkerServer {
    let res_cache = Arc::new(Mutex::new(ResourceCache::new(cache_cfg)));
    let num_workers = 1; // FIXME(20160113)
    //let (tx, rx) = channel();
    //let (tx, rx) = spmc::unbounded::new();
    let (tx, rx) = chan::async();
    let work_pool = ThreadPool::new(num_workers);
    for worker_idx in 0 .. num_workers {
      let work_rx = rx.clone();
      let work_res_cache = res_cache.clone();
      work_pool.execute(move || {
        let mut work_res_cache = work_res_cache;
        let mut work_queue = vec![];
        //let mut trial_queue = vec![];
        loop {
          let (trial_idx, experiment) = work_rx.recv().unwrap();
          work_queue.push((trial_idx, experiment));
          while !work_queue.is_empty() {
            let (trial_idx, experiment) = work_queue.pop().unwrap();
            let maybe_trial = {
              let mut work_res_cache = work_res_cache.lock().unwrap();
              Trial::create(trial_idx, &experiment, &mut *work_res_cache)
            };
            match maybe_trial {
              Some(trial) => {
                // TODO(20160113)
                create_dir(&trial.trial_path).ok();
                let mut procs = vec![];
                for &(ref exec, ref args, ref env_vars) in trial.programs.iter() {
                  let mut cmd = Command::new(&exec.dst_path);
                  for &(ref env_key, ref env_value) in env_vars.iter() {
                    cmd.env(env_key, env_value);
                  }
                  cmd.args(args);
                  cmd.current_dir(&trial.trial_path);
                  cmd.stdout(Stdio::piped());
                  cmd.stderr(Stdio::piped());
                  let child = match cmd.spawn() {
                    Ok(child) => child,
                    Err(e) => panic!("failed to start trial: {:?}", e),
                  };
                  procs.push(child);
                  sleep_ms(2000);
                }
                for child in procs.into_iter() {
                  // FIXME(20160113): instead of getting stdout/err at end,
                  // read it while running.
                  match child.wait_with_output() {
                    Ok(output) => {
                      let mut out_path = trial.trial_path.clone();
                      out_path.push(&format!("trial.{}.out", trial_idx));
                      let mut err_path = trial.trial_path.clone();
                      err_path.push(&format!("trial.{}.err", trial_idx));
                      let mut out_file = File::create(&out_path).unwrap();
                      out_file.write_all(&output.stdout).unwrap();
                      let mut err_file = File::create(&err_path).unwrap();
                      err_file.write_all(&output.stderr).unwrap();
                    }
                    Err(e) => panic!("failed to finish trial: {:?}", e),
                  }
                }
              }
              None => {
                work_queue.push((trial_idx, experiment));
                sleep_ms(1000);
              }
            }
          }
        }
      });
    }
    WorkerServer{
      //zmq_ctx:      zmq::Context::new(),
      res_cache:    res_cache,
      work_tx:      tx,
      work_pool:    work_pool,
    }
  }

  pub fn runloop(&mut self) {
    /*let mut sub = self.zmq_ctx.socket(zmq::SUB).unwrap();
    assert!(sub.connect(CONTROL_BCAST_ADDR).is_ok());
    assert!(sub.set_subscribe(&[]).is_ok());
    let mut receiver = self.zmq_ctx.socket(zmq::PULL).unwrap();
    assert!(receiver.connect(CONTROL_SOURCE_ADDR).is_ok());
    let mut sender = self.zmq_ctx.socket(zmq::PUSH).unwrap();
    assert!(sender.connect(CONTROL_SINK_ADDR).is_ok());*/
    let mut receiver = nanomsg::Socket::new(nanomsg::Protocol::Pull).unwrap();
    let receiver_end = receiver.connect(CONTROL_SOURCE_ADDR).unwrap();

    loop {
      //let encoded_bytes = receiver.recv_bytes(0).unwrap();
      let mut encoded_bytes = vec![];
      receiver.read_to_end(&mut encoded_bytes).unwrap();
      let msg: ProtocolMsg = {
        let encoded_str = from_utf8(&encoded_bytes).unwrap();
        json::decode(encoded_str).unwrap()
      };
      match msg {
        ProtocolMsg::PushWork{trial_idx, experiment} => {
          self.work_tx.send((trial_idx, experiment));
        }
        _ => {}
      }
    }
  }
}
