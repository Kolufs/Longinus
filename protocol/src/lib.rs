use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub identifier: u64,
    pub content: Option<Vec<u8>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Command {
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MessageError {
    InvalidIdentifier,
    InvalidContent,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Messages {
    Command(Command),
    MeasureBandwitch, // Variant for holding a Command
}

pub fn parse_message(message: Message) -> Result<Messages, MessageError> {
    match message.identifier {
        0 => match message.content {
            Some(content) => {
                let command: Result<Command, _> = serde_json::from_slice(&content);
                match command {
                    Ok(cmd) => Ok(Messages::Command(cmd)),
                    Err(_) => Err(MessageError::InvalidContent),
                }
            }
            None => Err(MessageError::InvalidContent),
        },
        1 => return Ok(Messages::MeasureBandwitch),
        _ => Err(MessageError::InvalidIdentifier),
    }
}
