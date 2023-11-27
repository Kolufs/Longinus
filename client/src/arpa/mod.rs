use pnet::ipnetwork::IpNetwork; 
use pnet_datalink::{MarcAddr, NetworkInterface};
use std::{fs, net::Ipv4Addr, str::FromStr};

enum ScanError {
}

pub struct Scanner {

}

impl Scanner {
    fn fetch_default_interface_from_proc() -> Option<pnet_datalink::NetworkInterface> {
        let route_info = fs::read_to_string("/proc/net/route").ok()?;

        for line in route_info.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts[1] == "00000000" && parts[7] == "00000000" {
                return pnet_datalink::interfaces()
                    .into_iter()
                    .find(|i| i.name == parts[0]);
            }
        }
        None
    }

    fn fetch_default_interface_from_pnet() -> Option<pnet_datalink::NetworkInterface> {
        pnet_datalink::interfaces()
            .into_iter()
            .find(|i| i.is_up() && !i.is_loopback() && !i.name.is_empty())
    }

    fn fetch_default_interface() -> Option<pnet_datalink::NetworkInterface> {
        Self::fetch_default_interface_from_proc().or_else(Self::fetch_default_interface_from_pnet)
    }

    fn scan(self) -> Result<Vec<MacAddr>, ScanError> {
        
    }
}
