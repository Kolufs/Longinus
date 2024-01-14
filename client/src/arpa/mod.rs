use pnet::ipnetwork::IpNetwork; 
use pnet::datalink::{MacAddr, self, NetworkInterface, DataLinkReceiver};
use pnet::packet::Packet;
use pnet::packet::ethernet::EtherType;
use pnet::packet::arp::ArpPacket; 
use pnet::packet::ethernet::EthernetPacket;
use std::borrow::BorrowMut;
use std::io;
use std::sync::{Arc, Mutex};
use std::ops::DerefMut;
use std::{fs, net::Ipv4Addr, str::FromStr, collections::HashSet};
use indexmap::IndexSet;

enum ScanError {
    InterfaceNotFound,
    ChannelCreationError(io::Error)
}

impl From<std::io::Error> for ScanError {
    fn from(value: std::io::Error) -> Self {
        ScanError::ChannelCreationError(value)
    }
}

pub struct SizedIndexSet<T: std::hash::Hash + Eq> {
    set: IndexSet<T>,
    size: usize, 
}

impl<T: std::hash::Hash + Eq> SizedIndexSet<T> {
    pub fn new(size: usize) -> Self {
        Self {
            set: IndexSet::new(),
            size, 
        }
    }

    pub fn insert(&mut self, element: T) -> bool {
        if self.set.len() == self.size {
            self.set.shift_remove_index(0); 
        }
        self.set.insert(element)
    }
}

pub struct Scanner {
    rx: Box<dyn DataLinkReceiver>,
    pub devices: Arc<Mutex<SizedIndexSet<MacAddr>>>,
}

impl Scanner {
    pub fn new() -> Result<Self, ScanError> {
        let interface = Self::fetch_default_interface().ok_or(ScanError::InterfaceNotFound)?;

        let channel: datalink::Channel = datalink::channel(&interface, Default::default())?;

        let (_, mut rx) = match channel {
            datalink::Channel::Ethernet(tx, rx) => (tx, rx),
            _ => todo!(),
        }; 

        Ok(Scanner {
            rx,
            devices: Arc::new(Mutex::new(SizedIndexSet::new(1024))),
        })
    }

    fn fetch_default_interface_from_proc() -> Option<NetworkInterface> {
        let route_info = fs::read_to_string("/proc/net/route").ok()?;

        for line in route_info.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts[1] == "00000000" && parts[7] == "00000000" {
                return datalink::interfaces()
                    .into_iter()
                    .find(|i| i.name == parts[0]);
            }
        }
        None
    }

    fn fetch_default_interface_from_pnet() -> Option<NetworkInterface> {
        datalink::interfaces()
            .into_iter()
            .find(|i| i.is_up() && !i.is_loopback() && !i.name.is_empty())
    }

    fn fetch_default_interface() -> Option<NetworkInterface> {
        Self::fetch_default_interface_from_proc().or_else(Self::fetch_default_interface_from_pnet)
    } 

    fn scan(mut self) {
        loop {
        match self.rx.next() {
            Ok(packet) => {
                let ethernet_packet = EthernetPacket::new(packet).unwrap();
                match ethernet_packet.get_ethertype() {
                    EtherType(0x806) =>  {
                        if let Some(packet) = ArpPacket::new(packet) {
                            let sender_addr = packet.get_sender_hw_addr();
                            (self.devices.try_lock().unwrap()).insert(sender_addr); 
                            let target_addr = packet.get_target_hw_addr(); 
                            if !([MacAddr::broadcast(), MacAddr::new(0,0,0,0,0,0)].contains(&target_addr)) {
                                (self.devices.try_lock().unwrap()).insert(target_addr);
                            };   
                            }
                        },
                    _ => ()
                }
            },  
            Err(_) => ()
        };
        }
    }
}
