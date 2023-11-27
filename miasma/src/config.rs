use serde::{Deserialize, Serialize};

use serde_json;
use std::env;
use std::fs::OpenOptions;
use std::io::BufReader;
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct ServerConfig {
    pub socket: SocketAddr,
    pub version: u64,
    pub binaries_folder: PathBuf,
    pub cert: PathBuf,
    pub key: PathBuf,
}

#[derive(Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub socket: SocketAddr,
    pub ns: String,
    pub db: String,
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub server: ServerConfig,
}

impl Config {
    pub fn load() -> Config {
        let file = OpenOptions::new()
            .read(true)
            .write(false)
            .create(false)
            .open(config_file)
            .unwrap();

        let reader = BufReader::new(file);

        serde_json::from_reader(reader).unwrap()
    }
}
