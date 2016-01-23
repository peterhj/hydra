use std::path::{PathBuf};

#[derive(Clone, RustcDecodable, RustcEncodable, Debug)]
pub struct Experiment {
  pub trial_cfg:    TrialConfig,
  pub current_path: PathBuf,
  pub trials_path:  PathBuf,
  pub num_trials:   usize,
}

#[derive(Clone, RustcDecodable, RustcEncodable, Debug)]
pub struct TrialConfig {
  //pub hyperparam:   Option<PathBuf>,
  pub resources:    Vec<(Resource, usize)>,
  pub assets:       Vec<Asset>,
  pub programs:     Vec<(PathBuf, Vec<String>, Vec<(String, String)>)>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, RustcDecodable, RustcEncodable, Debug)]
pub enum Resource {
  RandomSeed32,
  RandomSeed64,
  Port,
  CudaGpu,
}

impl Resource {
  pub fn to_key_string(&self, res_idx: usize) -> String {
    match self {
      &Resource::RandomSeed32 => {
        format!("HYDRA.SEED32.{}", res_idx)
      }
      &Resource::RandomSeed64 => {
        format!("HYDRA.SEED64.{}", res_idx)
      }
      &Resource::Port => {
        format!("HYDRA.PORT.{}", res_idx)
      }
      &Resource::CudaGpu => {
        format!("HYDRA.CUDAGPU.{}", res_idx)
      }
    }
  }
}

#[derive(Clone, RustcDecodable, RustcEncodable, Debug)]
pub enum Asset {
  Copy{src: PathBuf},
  Symlink{src: PathBuf},
  SymlinkAs{src: PathBuf, dst: PathBuf},
}
