use std::io::Write;

use anyhow::Result;
use clap::Args;

use crate::words_getter::WordGetter;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Args)]
pub struct Extend {
    /// Source file
    pub source_file: String,
}
impl Extend {
    pub fn new(source_file: String) -> Self {
        Self { source_file }
    }

    pub async fn extend_file(&self, words_fcontainer_name: &str) -> Result<()> {
        let mut words_to_add: Vec<String> = Vec::new();

        let mut file = None;
        let words = WordGetter::get_all_words(&self.source_file, &mut file).await?;

        for v in WordGetter::get_words_of_all_found_words(words_fcontainer_name, None)
            .await
            .unwrap()
            .iter()
        {
            for word in v.iter() {
                if !word.is_empty() && !words.contains(&word) && !words_to_add.contains(&word) {
                    words_to_add.push(word.to_owned());
                }
            }
        }

        for w in words_to_add.iter() {
            let mut data = w.as_bytes().to_vec();
            data.push(10);
            file.as_mut().unwrap().write(&data).unwrap();
        }

        println!("{} mots ont été ajoutés", words_to_add.len());

        Ok(())
    }
}
