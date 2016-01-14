#![feature(slice_bytes)]

extern crate chan;
//extern crate comm;
extern crate nanomsg;
extern crate rand;
extern crate rustc_serialize;
extern crate threadpool;
//extern crate zmq;

use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{channel};

pub mod api;
pub mod control;
pub mod server;
pub mod static_hosts;
pub mod worker;

/*pub struct Server;

impl Server {
  pub fn run(&mut self) {
    let listener = TcpListener::bind("127.0.0.1:9000").unwrap();
    for stream in listener.incoming() {
      match stream {
        Ok(stream) => {
          // TODO(20151201): accept a json experiment, parse it,
          // then enqueue it.
        }
        Err(e) => {
        }
      }
    }
  }
}*/
