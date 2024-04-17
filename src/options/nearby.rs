use std::io::{ErrorKind, Write};

use anyhow::Result;
use clap::Args;

use crate::words_getter::WordGetter;

use super::options::{Cli, LogLevel};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Args)]
pub struct Nearby {
    /// The word of the day only
    pub word: String,
}

impl Nearby {
    pub fn new(word: String) -> Self {
        Self { word }
    }

    pub async fn get_nearby(&self) -> anyhow::Result<String> {
        let client = reqwest::Client::new();
        let params = [("word", &self.word)];

        let a = client
            .post("https://cemantix.certitudes.org/nearby")
            .form(&params)
            .header("Content-type", "application/x-www-form-urlencoded");

        Ok(a.send().await?.text().await?)
    }

    pub async fn generate_nearby_word(&self, words_dir: &str, cli: &Cli) -> Result<()> {
        let file_content = self.get_nearby().await?;
        if file_content.is_empty() {
            return Err(anyhow::anyhow!(
                "Impossible de récupérer les mots proches de {}",
                self.word
            ));
        }
        let mut file_word =
            match WordGetter::get_file_word(&self.word, true, true, false, words_dir) {
                Ok(f) => f,
                Err(e) => {
                    match e.kind() {
                        ErrorKind::NotFound => {
                            return Err(anyhow::anyhow!(e));
                        }
                        ErrorKind::AlreadyExists => {
                            return Err(anyhow::anyhow!("Already generated"));
                        }
                        _ => {}
                    }
                    return Ok(());
                }
            };

        if let Err(e) = file_word.write(&file_content.as_bytes()) {
            cli.log_and_print(
                &format!("cannot write data to file '{}' ({e})", self.word),
                LogLevel::Error,
            )?;
        } else {
            cli.log_and_print(
                &format!("Successfully writen data into file '{}'", self.word),
                LogLevel::Info,
            )?;
        }

        Ok(())
    }
}
