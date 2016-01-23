use std::fs::{File};
use std::io::{BufRead, BufReader};
use std::path::{Path};

pub struct Hostfile {
  ctrl_cmd_ip:  String,
  ctrl_serv_ip: String,
}

impl Hostfile {
  pub fn new(hostfile_path: &Path) -> Hostfile {
    let hostfile = File::open(hostfile_path).unwrap();
    let reader = BufReader::new(hostfile);
    let mut ctrld_cmd_ip: Option<_> = None;
    let mut ctrld_source_ip: Option<_> = None;
    for (i, line) in reader.lines().enumerate() {
      match i {
        0 => {
          ctrld_cmd_ip = Some(line.unwrap());
        }
        1 => {
          ctrld_source_ip = Some(line.unwrap());
          break;
        }
        _ => unreachable!(),
      }
    }
    //panic!("hostfile {:?} missing ctrl server ip!", hostfile_path);
    Hostfile{
      ctrl_cmd_ip:    ctrld_cmd_ip.unwrap(),
      ctrl_serv_ip:   ctrld_source_ip.unwrap(),
    }
  }

  pub fn get_cmd_addr(&self) -> String {
    format!("tcp://{}:9999", self.ctrl_cmd_ip)
  }

  pub fn get_source_addr(&self) -> String {
    format!("tcp://{}:9001", self.ctrl_serv_ip)
  }
}
