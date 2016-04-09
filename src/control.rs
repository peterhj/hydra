use api::{Experiment};
use hosts::{Hostfile};
use server::{ProtocolMsg};

use nanomsg;
use rustc_serialize::json;
//use zmq;

use std::io::{Read, Write};
use std::str::{from_utf8};

#[derive(RustcDecodable, RustcEncodable)]
pub enum ControlCmd {
  Dummy,
  SubmitExperiment{experiment: Experiment},
}

pub struct ControlClient;

impl ControlClient {
  pub fn new() -> ControlClient {
    ControlClient
  }

  pub fn send_cmd(&mut self, cmd: ControlCmd) {
    /*let mut cmd_req = self.zmq_ctx.socket(zmq::REQ).unwrap();
    assert!(cmd_req.connect("tcp://127.0.0.1:9999").is_ok());
    let mut cmd_msg = zmq::Message::with_capacity(4096).unwrap();*/
    let mut cmd_req = nanomsg::Socket::new(nanomsg::Protocol::Req).unwrap();
    let cmd_req_end = cmd_req.connect("tcp://127.0.0.1:9999").unwrap();
    let encoded_str = json::encode(&cmd).unwrap();
    let encoded_bytes = (&encoded_str).as_bytes();
    println!("DEBUG: encoded: {:?}", encoded_str);
    cmd_req.write_all(encoded_bytes).unwrap();
    //let mut rep_bytes = vec![];
    //cmd_req.read_to_end(&mut rep_bytes).unwrap();
    let mut rep_str = String::new();
    cmd_req.read_to_string(&mut rep_str).unwrap();
    println!("DEBUG: received: {}", rep_str);
  }
}

pub struct ControlServer {
  hostfile: Hostfile,
}

impl ControlServer {
  pub fn new(hostfile: Hostfile) -> ControlServer {
    ControlServer{
      hostfile: hostfile,
    }
  }

  pub fn runloop(&mut self) {
    /*let mut cmd_reply = self.zmq_ctx.socket(zmq::REP).unwrap();
    assert!(cmd_reply.bind("tcp://127.0.0.1:9999").is_ok());
    let mut bcast = self.zmq_ctx.socket(zmq::PUB).unwrap();
    assert!(bcast.bind(CONTROL_BCAST_ADDR).is_ok());
    let mut source = self.zmq_ctx.socket(zmq::PUSH).unwrap();
    assert!(source.bind(CONTROL_SOURCE_ADDR).is_ok());
    let mut sink = self.zmq_ctx.socket(zmq::PULL).unwrap();
    assert!(sink.bind(CONTROL_SINK_ADDR).is_ok());*/

    println!("DEBUG: ctrld: binding cmd: {}",
        self.hostfile.get_cmd_addr());
    let mut cmd_rep = nanomsg::Socket::new(nanomsg::Protocol::Rep).unwrap();
    let mut cmd_rep_endpoint = cmd_rep.bind(&self.hostfile.get_cmd_addr()).unwrap();

    println!("DEBUG: ctrld: binding source: {}",
        self.hostfile.get_source_addr());
    let mut source = nanomsg::Socket::new(nanomsg::Protocol::Push).unwrap();
    let mut source_endpoint = source.bind(&self.hostfile.get_source_addr()).unwrap();

    let mut encoded_str = String::new();
    loop {
      //let mut encoded_bytes = vec![];
      //cmd_reply.read_to_end(&mut encoded_bytes).unwrap();
      encoded_str.clear();
      cmd_rep.read_to_string(&mut encoded_str).unwrap();
      let cmd: ControlCmd = {
        //let encoded_str = from_utf8(&encoded_bytes).unwrap();
        json::decode(&encoded_str).unwrap()
      };
      cmd_rep.write_all("ok".as_bytes()).unwrap();
      match cmd {
        ControlCmd::Dummy => {
          println!("DEBUG: received dummy message");
        }
        ControlCmd::SubmitExperiment{experiment} => {
          println!("DEBUG: received experiment");
          println!("DEBUG: {:?}", experiment);
          /*let msg = ProtocolMsg::NotifyWorkers;
          bcast.*/
          for trial_idx in 0 .. experiment.num_trials {
            let msg = ProtocolMsg::PushWork{
              trial_idx: trial_idx,
              experiment: experiment.clone(),
            };
            let encoded_str = json::encode(&msg).unwrap();
            let encoded_bytes = (&encoded_str).as_bytes();
            println!("DEBUG: sending trial {}/{}...",
                trial_idx, experiment.num_trials);
            source.write_all(encoded_bytes).unwrap();
          }
        }
      }
    }
    source_endpoint.shutdown().unwrap();

    /*println!("DEBUG: ctrld: binding source: {}",
        self.hostfile.get_source_addr());
    let mut source_rep = nanomsg::Socket::new(nanomsg::Protocol::Rep).unwrap();
    let mut source_rep_endpoint = source_rep.bind(&self.hostfile.get_source_addr()).unwrap();

    let mut encoded_str = String::new();
    loop {
      encoded_str.clear();
      cmd_rep.read_to_string(&mut encoded_str).unwrap();
      let cmd: ControlCmd = {
        json::decode(&encoded_str).unwrap()
      };
      cmd_rep.write_all("ok".as_bytes()).unwrap();

      match cmd {
        ControlCmd::Dummy => {
          println!("DEBUG: received dummy message");
        }
        ControlCmd::SubmitExperiment{experiment} => {
          println!("DEBUG: received experiment");
          println!("DEBUG: {:?}", experiment);
          for trial_idx in 0 .. experiment.num_trials {
            encoded_str.clear();
            source_rep.read_to_string(&mut encoded_str).unwrap();
            let recv_msg: ProtocolMsg = {
              json::decode(&encoded_str).unwrap()
            };

            match recv_msg {
              ProtocolMsg::RequestWork => {
                let msg = ProtocolMsg::PushWork{
                  trial_idx: trial_idx,
                  experiment: experiment.clone(),
                };
                let encoded_msg = json::encode(&msg).unwrap();
                let encoded_bytes = encoded_msg.as_bytes();
                println!("DEBUG: sending trial {}/{}...",
                    trial_idx, experiment.num_trials);
                source_rep.write_all(encoded_bytes).unwrap();
              }
              _ => unimplemented!(),
            }
          }
        }
      }
    }
    source_rep_endpoint.shutdown().unwrap();*/

    cmd_rep_endpoint.shutdown().unwrap();
  }
}
