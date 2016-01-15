use api::{Experiment};
use server::{ProtocolMsg};
use static_hosts::{CONTROL_BCAST_ADDR, CONTROL_SOURCE_ADDR, CONTROL_SINK_ADDR};

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
    let mut rep_bytes = vec![];
    cmd_req.read_to_end(&mut rep_bytes).unwrap();
  }
}

pub struct ControlServer;

impl ControlServer {
  pub fn new() -> ControlServer {
    ControlServer
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
    let mut cmd_reply = nanomsg::Socket::new(nanomsg::Protocol::Rep).unwrap();
    let cmd_reply_end = cmd_reply.bind("tcp://127.0.0.1:9999").unwrap();
    let mut source = nanomsg::Socket::new(nanomsg::Protocol::Rep).unwrap();
    let source_end = cmd_reply.bind(CONTROL_SOURCE_ADDR).unwrap();

    loop {
      //let encoded_bytes = cmd_reply.recv_bytes(0).unwrap();
      let mut encoded_bytes = vec![];
      cmd_reply.read_to_end(&mut encoded_bytes).unwrap();
      let cmd: ControlCmd = {
        let encoded_str = from_utf8(&encoded_bytes).unwrap();
        json::decode(encoded_str).unwrap()
      };
      //cmd_reply.send(&[], 0).unwrap();
      cmd_reply.write_all(&[]).unwrap();
      match cmd {
        ControlCmd::Dummy => {}
        ControlCmd::SubmitExperiment{experiment} => {
          /*let msg = ProtocolMsg::NotifyWorkers;
          bcast.*/
          for trial_idx in 0 .. experiment.num_trials {
            let msg = ProtocolMsg::PushWork{
              trial_idx: trial_idx,
              experiment: experiment.clone(),
            };
            let encoded_str = json::encode(&msg).unwrap();
            let encoded_bytes = (&encoded_str).as_bytes();
            //source.send(encoded_bytes, 0).unwrap();
            source.write_all(encoded_bytes).unwrap();
          }
        }
      }
    }
  }
}
