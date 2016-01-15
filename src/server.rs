use api::{Resource, Experiment, Asset};

use rand::{Rng, thread_rng};
use std::collections::{HashMap, HashSet};
use std::ops::{Range};
use std::path::{PathBuf};
use std::thread::{sleep_ms};

pub enum ResourceValue {
  RandomSeed32{seed: u32},
  RandomSeed64{seed: u64},
  Port{port: u16},
  CudaGpu{dev_idx: usize},
}

impl ResourceValue {
  pub fn to_arg_string(&self) -> String {
    match self {
      &ResourceValue::RandomSeed32{seed} => {
        format!("{}", seed)
      }
      &ResourceValue::RandomSeed64{seed} => {
        format!("{}", seed)
      }
      &ResourceValue::Port{port} => {
        format!("{}", port)
      }
      _ => unreachable!(),
    }
  }
}

pub struct DualPath {
  src_path:     PathBuf,
  pub dst_path: PathBuf,
}

impl DualPath {
  pub fn new(path: &PathBuf, trials_path: &PathBuf) -> DualPath {
    let mut dst_path = trials_path.clone();
    dst_path.set_file_name(&path.file_name().unwrap());
    DualPath{
      src_path: path.clone(),
      dst_path: dst_path,
    }
  }
}

fn expand_env_vars(/*hyperparam: Option<&DualPath>, */env_vars: &[(String, String)], resource_map: &HashMap<(Resource, usize), ResourceValue>) -> Option<Vec<(String, String)>> {
  let mut expand_env_vars = vec![];

  /*if let Some(hyperparam) = hyperparam {
    expand_env_vars.push(("HYPERPARAM_PATH".to_string(), hyperparam.dst_path.to_str().unwrap().to_string()));
  }*/

  for &(ref env_key, ref env_value) in env_vars.iter() {
    // FIXME(20160115): replace variable that is trial working path.
    /*let mut expand_value = env_value.clone();*/
    expand_env_vars.push((env_key.clone(), env_value.clone()));
  }

  let mut dev_idxs = vec![];
  for (&(resource, res_idx), res_val) in resource_map.iter() {
    match resource {
      Resource::CudaGpu => {
        match res_val {
          &ResourceValue::CudaGpu{dev_idx} => {
            dev_idxs.push(dev_idx);
          }
          _ => unreachable!(),
        }
      }
      _ => {}
    }
  }
  if !dev_idxs.is_empty() {
    dev_idxs.sort();
    let dev_idx_strs: Vec<String> = dev_idxs.iter()
      .map(|&dev_idx| format!("{}", dev_idx))
      .collect();
    expand_env_vars.push(("CUDA_VISIBLE_DEVICES".to_string(), dev_idx_strs.join(",")));
  }

  Some(expand_env_vars)
}

fn expand_args(args: &[String], resource_map: &HashMap<(Resource, usize), ResourceValue>) -> Option<Vec<String>> {
  let mut expand_args = Vec::with_capacity(args.len());
  for arg in args.iter() {
    for (&(resource, res_idx), res_val) in resource_map.iter() {
      match resource {
        Resource::RandomSeed32 => {
          let new_arg = arg.replace(&format!("${{HYPER.SEED32.{}}}", res_idx), &res_val.to_arg_string());
          expand_args.push(new_arg);
        }
        Resource::RandomSeed64 => {
          let new_arg = arg.replace(&format!("${{HYPER.SEED64.{}}}", res_idx), &res_val.to_arg_string());
          expand_args.push(new_arg);
        }
        Resource::Port => {
          let new_arg = arg.replace(&format!("${{HYPER.PORT.{}}}", res_idx), &res_val.to_arg_string());
          expand_args.push(new_arg);
        }
        _ => {}
      }
    }
    // TODO(20160112)
    unimplemented!();
  }
  Some(expand_args)
}

pub struct Trial {
  pub trial_idx:    usize,
  pub resource_map: HashMap<(Resource, usize), ResourceValue>,
  pub trial_path:   PathBuf,
  //pub hyperparam:   Option<DualPath>,
  //pub env_vars:     Vec<(String, String)>,
  pub assets:       Vec<DualPath>,
  pub programs:     Vec<(DualPath, Vec<String>, Vec<(String, String)>)>,
}

impl Trial {
  pub fn create(trial_idx: usize, experiment: &Experiment, cache: &mut ResourceCache) -> Option<Trial> {
    let mut resource_map = HashMap::new();
    for &(resource, res_count) in experiment.trial_cfg.resources.iter() {
      for res_idx in 0 .. res_count {
        match cache.allocate(&resource) {
          Some(value) => {
            resource_map.insert((resource, res_idx), value);
          }
          None => {
            cache.reclaim(&resource_map);
            return None;
          }
        }
      }
    }
    /*let hyperparam = experiment.trial_cfg.hyperparam
      .as_ref().map(|p| DualPath::new(p, &experiment.trials_path));*/
    let assets: Vec<_> = experiment.trial_cfg.assets
      .iter().map(|asset| match asset {
        &Asset::Copy{ref src} => {
          unimplemented!();
        }
        &Asset::Symlink{ref src} => {
          unimplemented!();
        }
      })
      .collect();
    let programs: Vec<_> = experiment.trial_cfg.programs
      .iter().map(|&(ref p, ref args, ref env_vars)| {
        let args = match expand_args(args, &resource_map) {
          Some(args) => args,
          None => panic!("invalid args: {:?}", args),
        };
        let env_vars = match expand_env_vars(env_vars, &resource_map) {
          Some(env_vars) => env_vars,
          None => panic!("failed to make env vars!"),
        };
        (DualPath::new(p, &experiment.trials_path), args, env_vars)
      })
      .collect();
    Some(Trial{
      trial_idx:    trial_idx,
      resource_map: resource_map,
      trial_path:   experiment.trials_path.clone(),
      assets:       assets,
      programs:     programs,
    })
  }
}

pub struct ResourceCacheConfig {
  pub port_range:       Range<u16>,
  pub dev_idx_range:    Range<usize>,
}

pub struct ResourceCache {
  avail_ports:      Vec<u16>,
  used_ports:       HashSet<u16>,
  avail_dev_idxs:   Vec<usize>,
  used_dev_idxs:    HashSet<usize>,
}

impl ResourceCache {
  pub fn new(cfg: ResourceCacheConfig) -> ResourceCache {
    let mut avail_ports = cfg.port_range.collect();
    let mut avail_dev_idxs = cfg.dev_idx_range.collect();
    ResourceCache{
      avail_ports:      avail_ports,
      used_ports:       HashSet::new(),
      avail_dev_idxs:   avail_dev_idxs,
      used_dev_idxs:    HashSet::new(),
    }
  }

  pub fn allocate(&mut self, resource: &Resource) -> Option<ResourceValue> {
    match resource {
      &Resource::RandomSeed32 => {
        Some(ResourceValue::RandomSeed32{seed: thread_rng().next_u32()})
      }
      &Resource::RandomSeed64 => {
        Some(ResourceValue::RandomSeed64{seed: thread_rng().next_u64()})
      }
      &Resource::Port => {
        if self.avail_ports.is_empty() {
          return None;
        }
        let num_avail = self.avail_ports.len();
        let port = self.avail_ports.swap_remove(num_avail - 1);
        self.used_ports.insert(port);
        Some(ResourceValue::Port{port: port})
      }
      &Resource::CudaGpu => {
        if self.avail_dev_idxs.is_empty() {
          return None;
        }
        let num_avail = self.avail_dev_idxs.len();
        let dev_idx = self.avail_dev_idxs.swap_remove(num_avail - 1);
        self.used_dev_idxs.insert(dev_idx);
        Some(ResourceValue::CudaGpu{dev_idx: dev_idx})
      }
    }
  }

  pub fn reclaim(&mut self, resource_map: &HashMap<(Resource, usize), ResourceValue>) {
    for (&(resource, res_idx), res_val) in resource_map.iter() {
      match resource {
        Resource::Port => {
          match res_val {
            &ResourceValue::Port{port} => {
              assert!(self.used_ports.contains(&port));
              self.used_ports.remove(&port);
              self.avail_ports.push(port);
            }
            _ => unreachable!(),
          }
        }
        Resource::CudaGpu => {
          match res_val {
            &ResourceValue::CudaGpu{dev_idx} => {
              assert!(self.used_dev_idxs.contains(&dev_idx));
              self.used_dev_idxs.remove(&dev_idx);
              self.avail_dev_idxs.push(dev_idx);
            }
            _ => unreachable!(),
          }
        }
        _ => {}
      }
    }
  }
}

#[derive(RustcDecodable, RustcEncodable)]
pub enum ProtocolMsg {
  NotifyWorkers,
  NotifyEndOfWork,
  AckNotify,
  RequestWork,
  PushWork{trial_idx: usize, experiment: Experiment},
}
