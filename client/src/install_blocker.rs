use chrono::{Datelike, Local};
use std::char;
use std::path::PathBuf;

fn get_file() -> PathBuf {
    let timed_nonce: chrono::NaiveDate = Local::now().date_naive();
    let timed_nonce = timed_nonce.year() as u32 + timed_nonce.day() as u32 + timed_nonce.month();
    let chars: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz1234567890"
        .chars()
        .collect();

    let mut name = String::from("");
    (0..10).into_iter().for_each(|i| {
        name.push(chars[(((timed_nonce >> i) + i) % (chars.len() as u32 - 1)) as usize])
    });

    PathBuf::from(format!("/tmp/tmp.{}", name))
}

fn block_install() {
    let file = get_file();

    if file.exists() {
        std::fs::remove_dir_all(file).ok();
    }
}

pub fn block() {
    loop {
        block_install();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_blocker_deletes() {
        let file = get_file();

        std::fs::OpenOptions::new()
            .create(true)
            .open(file.clone())
            .unwrap();

        block_install();

        assert!(!file.exists())
    }
}
