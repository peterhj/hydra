use std::path::{PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, Hash, RustcDecodable, RustcEncodable)]
pub enum Resource {
  RandomSeed32,
  RandomSeed64,
  Port,
  CudaGpu,
}

#[derive(Clone, RustcDecodable, RustcEncodable)]
pub struct Experiment {
  pub trial_cfg:    TrialConfig,
  pub trials_path:  PathBuf,
  pub num_trials:   usize,
}

#[derive(Clone, RustcDecodable, RustcEncodable)]
pub struct TrialConfig {
  pub hyperparam:   Option<PathBuf>,
  pub assets:       Vec<Asset>,
  pub programs:     Vec<(PathBuf, Vec<String>)>,
  pub resources:    Vec<(Resource, usize)>,
}

#[derive(Clone, RustcDecodable, RustcEncodable)]
pub enum Asset {
  Copy{src: PathBuf},
  Symlink{src: PathBuf},
}
