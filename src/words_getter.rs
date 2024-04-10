use std::{
    fs::{self, read_dir, read_to_string, OpenOptions},
    io::{BufRead, BufReader},
    path::PathBuf,
};

use anyhow::Result;
use chrono::NaiveDate;

use crate::cemantix_word::CemantixWord;
pub struct WordGetter {}

impl WordGetter {
    pub async fn get_all_words(
        filename: &str,
        file_init: &mut Option<fs::File>,
    ) -> Result<Vec<String>> {
        let mut words: Vec<String> = Vec::new();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(filename)?;

        let reader = BufReader::new(file.try_clone().unwrap());
        let _ = file_init.insert(file);
        for (_i, line) in reader.lines().enumerate() {
            let line: String = line.unwrap();
            words.push(line);
        }

        Ok(words)
    }
    pub async fn get_words_of_all_found_words(
        words_fcontainer_name: &str,
        found_words: Option<&[String]>,
    ) -> Result<Vec<Vec<String>>> {
        let mut words_list: Vec<Vec<String>> = Vec::new();
        let mut f: Option<Vec<String>> = None;
        if found_words.is_none() {
            f = Some(Self::get_all_found_word(words_fcontainer_name)?);
        }

        for word in match found_words {
            Some(v) => v,
            None => match &f {
                Some(v) => v,
                None => unreachable!(),
            },
        }
        .iter()
        {
            words_list.push(Self::get_words_of_found_word(word, words_fcontainer_name).await?);
        }

        Ok(words_list)
    }
    pub async fn get_words_of_found_word(
        word: &str,
        words_fcontainer_name: &str,
    ) -> Result<Vec<String>> {
        Ok(
            Self::get_cemantix_words_of_found_word(word, words_fcontainer_name)?
                .into_iter()
                .map(|c| c.word.to_owned())
                .collect(),
        )
    }
    pub fn get_cemantix_words_of_found_word(
        word: &str,
        words_fcontainer_name: &str,
    ) -> Result<Vec<CemantixWord>> {
        let file_content = read_to_string(PathBuf::from(words_fcontainer_name).join(word))?;
        Ok(serde_json::from_str::<Vec<CemantixWord>>(&file_content)?)
    }

    pub fn get_all_found_words_except(
        words_fcontainer_name: &str,
        words_exception: &[&String],
    ) -> Result<Vec<String>> {
        let mut data = Self::get_all_found_word(words_fcontainer_name)?;
        data.retain(|v| !words_exception.contains(&v));
        Ok(data)
    }
    /// returns the file which stores the closest word of a found word
    pub fn get_file_word(
        word: &str,
        create_new: bool,
        write: bool,
        append: bool,
        words_fcontainer_name: &str,
    ) -> Result<std::fs::File, std::io::Error> {
        // folder container does not exist
        if let Err(_) = fs::read_dir(words_fcontainer_name) {
            fs::create_dir(words_fcontainer_name)?;
        }

        let file = OpenOptions::new()
            .create_new(create_new)
            .append(append)
            .write(write)
            .read(true)
            .open(PathBuf::from(words_fcontainer_name).join(&word.clone()))?;
        return Ok(file);
    }
    pub fn get_all_found_word(words_fcontainer_name: &str) -> Result<Vec<String>> {
        Ok(read_dir(words_fcontainer_name)?
            .map(|f| match f {
                Ok(file) => {
                    if file.file_type().unwrap().is_file() {
                        return file.file_name().to_str().unwrap().to_string();
                    }
                    String::from("")
                }
                Err(_) => String::from(""),
            })
            .filter(|v| !v.eq(""))
            .collect::<Vec<String>>())
    }
    pub fn get_last_found_word(word_history_filename: &str) -> Result<Option<(String, NaiveDate)>> {
        let file = OpenOptions::new()
            .create(false)
            .read(true)
            .open(word_history_filename)?;
        let line = BufReader::new(file).lines().last();
        match line {
            Some(l) => {
                let line = l?;
                let mut data = line.split(":");
                let word = data.next().map(|v| v.trim().to_string());
                let date = data
                    .next()
                    .map(|v| NaiveDate::parse_from_str(v.trim(), crate::HISTORY_FORMAT));
                if let (Some(w), Some(d)) = (word, date) {
                    Ok(Some((w, d?)))
                } else {
                    Err(anyhow::anyhow!("Invalid end of file"))
                }
            }
            None => Ok(None),
        }
    }
}
