extern crate hydra;

use hydra::api::{Experiment, TrialConfig, Resource, Asset};
use hydra::control::{ControlClient, ControlCmd};

use std::path::{PathBuf};

fn main() {
  let trial_cfg = TrialConfig{
    resources:  vec![
      (Resource::RandomSeed32, 1),
      (Resource::Port, 1),
      (Resource::CudaGpu, 1),
    ],
    assets:     vec![
      Asset::Copy{src: PathBuf::from("hello.copy")},
      Asset::Symlink{src: PathBuf::from("hello.symlink")},
      Asset::SymlinkAs{
        src: PathBuf::from("hello.symlink"),
        dst: PathBuf::from("world.symlink"),
      },
    ],
    programs:   vec![
      (PathBuf::from("/bin/echo"),
        vec!["Hello world".to_string()],
        vec![]),
    ],
  };
  let experiment = Experiment{
    trial_cfg:      trial_cfg,
    trials_path:    PathBuf::from("trials"),
    scratch_prefix: PathBuf::from("/scratch/phj/space/holmes-project/test"),
    num_trials:     10,
  };
  let mut client = ControlClient::new();
  client.send_cmd(ControlCmd::SubmitExperiment{experiment: experiment});
}
