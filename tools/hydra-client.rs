extern crate hydra;

use hydra::control::{ControlCmd, ControlClient};

fn main() {
  let mut client = ControlClient::new();
  client.send_cmd(ControlCmd::Dummy);
}
