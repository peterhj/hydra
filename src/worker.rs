use api::{Experiment};
use hosts::{Hostfile};
use server::{Trial, DualAsset, ResourceCacheConfig, ResourceCache, ProtocolMsg};

//use comm::spmc;
use chan;
use nanomsg;
use rustc_serialize::json;
use threadpool::{ThreadPool};
//use zmq;

use std::env::{current_dir, set_current_dir};
use std::fs::{File, canonicalize, copy, create_dir_all};
use std::io::{Read, Write};
use std::os::unix::fs::{symlink};
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::process::{Command, Stdio};
use std::str::{from_utf8};
use std::sync::{Arc, Mutex};
//use std::sync::mpsc::{Sender, channel};
use std::thread::{sleep_ms};

/*enum State {
  Idle,
}*/

fn work_loop(work_rx: chan::Receiver<(usize, Experiment)>, work_res_cache: Arc<Mutex<ResourceCache>>, work_lock: Arc<Mutex<()>>) {
  let orig_current_path = current_dir().unwrap();
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
          let mut procs = vec![];
          //let mut stdouts = vec![];
          //let mut stderrs = vec![];

          {
            let guard = work_lock.lock().unwrap();

            // Change our current directory.
            //assert!(set_current_dir(&trial.current_path).is_ok());

            // Create the trial output directory.
            match create_dir_all(&trial.output_path) {
              Ok(_) => {}
              Err(_) => assert!(trial.output_path.is_dir()),
            }

            // Create the trial scratch directory.
            match create_dir_all(&trial.scratch_path) {
              Ok(_) => {}
              Err(_) => assert!(trial.scratch_path.is_dir()),
            }

            // Dump resource map to file (looks like variable definitions).
            {
              let mut resmap_path = trial.scratch_path.clone();
              resmap_path.push(&format!("trial.{}.resmap", trial_idx));
              let mut resmap_file = File::create(&resmap_path).unwrap();
              let mut resmap_kv_pairs = vec![];
              for (&(resource, res_idx), res_value) in trial.resource_map.iter() {
                //writeln!(resmap_file, "{} = \"{}\"", resource.to_key_string(res_idx), res_value.to_value_string()).unwrap();
                resmap_kv_pairs.push((resource.to_key_string(res_idx), res_value.to_value_string()));
              }
              resmap_kv_pairs.sort();
              for &(ref key, ref value) in resmap_kv_pairs.iter() {
                writeln!(resmap_file, "\"{}\" = \"{}\"", key, value);
              }
            }

            // Create asset files.
            for asset in trial.assets.iter() {
              match asset {
                &DualAsset::Copy{ref path} => {
                  // FIXME(20160122): create missing dirs.
                  let mut dst_dir = path.dst_path.clone();
                  dst_dir.pop();
                  match create_dir_all(&dst_dir) {
                    Ok(_) => {}
                    Err(_) => assert!(dst_dir.is_dir()),
                  }
                  /*println!("DEBUG: worker: copy asset: {:?} -> {:?}",
                      path.src_path, path.dst_path);*/
                  if !path.dst_path.exists() {
                    copy(&path.src_path, &path.dst_path).unwrap();
                  }
                }
                &DualAsset::Symlink{ref path} => {
                  // FIXME(20160122): create missing dirs.
                  let mut dst_dir = path.dst_path.clone();
                  dst_dir.pop();
                  match create_dir_all(&dst_dir) {
                    Ok(_) => {}
                    Err(_) => assert!(dst_dir.is_dir()),
                  }
                  /*println!("DEBUG: worker: symlink asset: {:?} -> {:?}",
                      path.src_path, path.dst_path);*/
                  if !path.dst_path.exists() {
                    symlink(&path.src_path, &path.dst_path).unwrap();
                  }
                }
              }
            }

            // Copy process executables.
            for &(ref exec, ref args, ref env_vars) in trial.programs.iter() {
              if !exec.dst_path.exists() {
                copy(&exec.src_path, &exec.dst_path).unwrap();
              }
            }

            // Dump shell commands to scripts.
            {
              let mut script_path = trial.scratch_path.clone();
              script_path.push(&format!("trial.{}.sh", trial_idx));
              let mut script_file = File::create(&script_path).unwrap();
              writeln!(script_file, "#!/bin/sh").unwrap();
              for &(ref exec, ref args, ref env_vars) in trial.programs.iter() {
                for &(ref env_key, ref env_value) in env_vars.iter() {
                  write!(script_file, "{}={} ", env_key, env_value).unwrap();
                }
                write!(script_file, "{} ", canonicalize(&exec.dst_path).unwrap().as_os_str().to_str().unwrap()).unwrap();
                for arg in args.iter() {
                  write!(script_file, "{} ", arg).unwrap();
                }
                writeln!(script_file, "").unwrap();
              }
            }
          }

          // Launch processes.
          for (proc_idx, &(ref exec, ref args, ref env_vars)) in trial.programs.iter().enumerate() {
            if !exec.dst_path.exists() {
              copy(&exec.src_path, &exec.dst_path).unwrap();
            }
            let mut cmd = Command::new(&canonicalize(&exec.dst_path).unwrap());
            for &(ref env_key, ref env_value) in env_vars.iter() {
              cmd.env(env_key, env_value);
            }
            cmd.args(args);
            cmd.current_dir(&trial.scratch_path);

            // FIXME(20160123): try to pipe stdout/stderr directly to files,
            // otherwise the buffer can fill up (looking at you, pachi-11)
            // and will hang the process.
            /*cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());*/

            let mut out_path = trial.output_path.clone();
            out_path.push(&format!("trial.{}.out.{}", trial_idx, proc_idx));
            let out_file = File::create(&out_path).unwrap();
            unsafe { cmd.stdout(Stdio::from_raw_fd(out_file.into_raw_fd())) };

            let mut err_path = trial.output_path.clone();
            err_path.push(&format!("trial.{}.err.{}", trial_idx, proc_idx));
            let err_file = File::create(&err_path).unwrap();
            unsafe { cmd.stderr(Stdio::from_raw_fd(err_file.into_raw_fd())) };

            let child = match cmd.spawn() {
              Ok(child) => child,
              Err(e) => panic!("failed to start trial: {:?}", e),
            };
            procs.push(child);
            // FIXME(20160115): should be a configurable delay for stability.
            sleep_ms(3000);
          }

          // Join processes.
          for (proc_idx, mut child) in procs.into_iter().enumerate() {
            // FIXME(20160113): instead of getting stdout/err at end, read and
            // dump them periodically; can do this by .take()'ing stdout and
            // stderr and reading them from another thread.
            /*match child.wait_with_output() {
              Ok(output) => {
                let mut out_path = trial.output_path.clone();
                out_path.push(&format!("trial.{}.out.{}", trial_idx, proc_idx));
                let mut err_path = trial.output_path.clone();
                err_path.push(&format!("trial.{}.err.{}", trial_idx, proc_idx));
                let mut out_file = File::create(&out_path).unwrap();
                out_file.write_all(&output.stdout).unwrap();
                let mut err_file = File::create(&err_path).unwrap();
                err_file.write_all(&output.stderr).unwrap();
              }
              Err(e) => panic!("failed to finish trial: {:?}", e),
            }*/
            match child.wait() {
              Ok(_) => {}
              Err(e) => panic!("failed to finish trial: {:?}", e),
            }
          }

          // Reclaim resources.
          let mut work_res_cache = work_res_cache.lock().unwrap();
          work_res_cache.reclaim(&trial.resource_map);

          // Reset our current directory.
          //assert!(set_current_dir(&orig_current_path).is_ok());
        }

        None => {
          work_queue.push((trial_idx, experiment));
          sleep_ms(1000);
        }
      }
    }
  }
}

pub struct WorkerServer {
  hostfile:     Hostfile,
  res_cache:    Arc<Mutex<ResourceCache>>,
  //work_tx:      Sender<(usize, Experiment)>,
  //work_tx:      spmc::unbounded::Producer<'a, (usize, Experiment)>,
  work_tx:      chan::Sender<(usize, Experiment)>,
  work_pool:    ThreadPool,
}

impl WorkerServer {
  pub fn new(num_workers: usize, hostfile: Hostfile, cache_cfg: ResourceCacheConfig) -> WorkerServer {
    let res_cache = Arc::new(Mutex::new(ResourceCache::new(cache_cfg)));
    //let num_workers = 1; // FIXME(20160113)
    //let (tx, rx) = channel();
    //let (tx, rx) = spmc::unbounded::new();
    let (tx, rx) = chan::async();
    let work_pool = ThreadPool::new(num_workers);
    let work_lock = Arc::new(Mutex::new(()));
    for worker_idx in 0 .. num_workers {
      let work_rx = rx.clone();
      let work_res_cache = res_cache.clone();
      let work_lock = work_lock.clone();
      work_pool.execute(move || {
        work_loop(work_rx, work_res_cache, work_lock);
      });
    }
    WorkerServer{
      hostfile:     hostfile,
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

    println!("DEBUG: worker: connecting to source: {}",
        self.hostfile.get_source_addr());
    let mut source_recv = nanomsg::Socket::new(nanomsg::Protocol::Pull).unwrap();
    source_recv.connect(&self.hostfile.get_source_addr()).unwrap();

    let mut encoded_str = String::new();
    loop {
      //let mut encoded_bytes = vec![];
      //source_recv.read_to_end(&mut encoded_bytes).unwrap();
      encoded_str.clear();
      source_recv.read_to_string(&mut encoded_str).unwrap();
      println!("DEBUG: worker: received message: {}", encoded_str);
      let msg: ProtocolMsg = {
        //let encoded_str = from_utf8(&encoded_bytes).unwrap();
        json::decode(&encoded_str).unwrap()
      };
      match msg {
        ProtocolMsg::PushWork{trial_idx, experiment} => {
          println!("DEBUG: worker: received work: trial: {} experiment: {:?}",
              trial_idx, experiment);
          self.work_tx.send((trial_idx, experiment));
        }
        _ => unimplemented!(),
      }
    }
  }
}
