use anyhow::Result;
use clap::Args;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    process::Command,
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Args)]
pub struct Sort {
    /// source file
    pub source_file: String,
}

impl Sort {
    pub async fn sort_file(&self, filename: &str) -> Result<()> {
        let output = Command::new("sort").args([filename]).output()?.stdout;

        let mut i = 1;
        let mut file_exists = true;
        let mut new_filename = String::from("");

        while file_exists {
            new_filename = filename.to_owned() + &i.to_string();
            match OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(new_filename.to_owned())
            {
                Ok(mut f) => {
                    f.write_all(&output)?;
                    file_exists = false;
                }
                Err(_) => {
                    i += 1;
                    continue;
                }
            };
        }

        fs::remove_file(filename).unwrap();
        fs::rename(new_filename, filename).unwrap();

        Ok(())
    }
}
