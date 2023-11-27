use bytes::Bytes;
use nix::sys::memfd::{memfd_create, MemFdCreateFlag};
use nix::unistd::execv;
use std::fs::File;

use std::ffi::CString;
use std::io::Write;
use std::os::fd::{AsFd, AsRawFd, FromRawFd};
use std::path::PathBuf;

use protocol::{parse_message, Command, Message, MessageError, Messages};
use std::process;

enum MessageIdentifier {
    Command,
    Update,
}

pub fn update(program: Bytes) {
    let program = program.into_iter().collect::<Vec<u8>>();

    let fd = memfd_create(&CString::new("").unwrap(), MemFdCreateFlag::MFD_CLOEXEC)
        .unwrap()
        .as_raw_fd();
    let mut file = unsafe { File::from_raw_fd(fd) };

    file.write_all(&program[..]).unwrap();

    let path = format!("/proc/{}/fd/{}", process::id().to_string(), fd.to_string());
    let path = CString::new(path).unwrap();

    let args = CString::new("").unwrap();

    execv(&path, &[&args]).unwrap();
}

fn handle_command(command: Command) {
    let comm = std::process::Command::new(command.command)
        .args(command.args)
        .spawn();
}

fn handle_measure_bandwitch() {}

pub fn handle_message(message: Message) {
    let message = match parse_message(message) {
        Ok(message) => message,
        Err(_) => return,
    };

    match message {
        Messages::Command(command) => handle_command(command),
        Messages::MeasureBandwitch => handle_measure_bandwitch(),
    }
}
