use anyhow::Result;
use clap::Args;
use std::{
    fs::{self, OpenOptions},
    io::Write,
};

use crate::words_getter::WordGetter;

use super::options::Cli;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Args)]
pub struct Sort {
    /// represent a file that stores a word at each line => words to be tested
    #[arg(long, default_value_t = String::new())]
    words_list_file: String,

    /// represents a file that stores the words with their temperature serialized by the program depending on a found word
    #[arg(long, default_value_t = String::new())]
    found_word_filename: String,
}

impl Sort {
    fn match_file(&self, words_fcontainer: &str) -> Result<(String, String)> {
        if self.found_word_filename.is_empty() && self.words_list_file.is_empty() {
            return Err(anyhow::anyhow!(
                "Error: wrong arguements, please provide words_list_file or found_word_file"
            ));
        } else if !self.words_list_file.is_empty() && !self.words_list_file.is_empty() {
            return Err(anyhow::anyhow!("Error: wrong arguements, please provide only one of words_list_file or found_word_file"));
        }

        if !self.found_word_filename.is_empty() {
            let mut words = WordGetter::get_cemantix_words_of_found_word(
                &self.found_word_filename,
                words_fcontainer,
            )?;
            words.sort();
            words.reverse();
            Ok((
                serde_json::to_string(&words)?,
                words_fcontainer.to_owned() + &self.found_word_filename,
            ))
        } else if !self.words_list_file.is_empty() {
            let file_content = std::fs::read_to_string(&self.words_list_file)?;
            let mut words = file_content
                .split("\n")
                .map(|v| v.to_string())
                .collect::<Vec<String>>();
            words.sort();
            words.reverse();
            Ok((words.join("\n"), self.words_list_file.to_owned()))
        } else {
            unreachable!()
        }
    }

    pub async fn sort_file(&self, cli: &Cli) -> Result<()> {
        let mut i = 1;
        let mut file_exists = true;
        let mut new_filename = String::new();
        let (data, filename) = self.match_file(&cli.words_directory)?;

        // loop creating a copy of the file but with filemame.ext{i}
        while file_exists {
            new_filename = filename.to_owned() + &i.to_string();
            match OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&new_filename)
            {
                Ok(mut f) => {
                    f.write_all(&data.as_bytes())?;
                    file_exists = false;
                }
                Err(_) => {
                    i += 1;
                    continue;
                }
            };
        }

        fs::remove_file(&filename)?;
        fs::rename(new_filename, &filename)?;

        Ok(())
    }
}
