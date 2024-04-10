use std::{collections::HashSet, sync::Arc};

use anyhow::Result;
use chrono::Local;
use clap::Args;
use futures::lock::Mutex;

use crate::{cemantix_word::CemantixWord, utils::send_words, words_getter::WordGetter};

use super::{options::Cli, solve::SolverStruct};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Args)]
pub struct Graph {
    /// Number of words in batches not over 200
    #[arg(short, long, default_value_t = 100)]
    pub batch_size: usize,
}

impl Graph {
    pub fn new(batch_size: usize) -> Self {
        Self { batch_size }
    }

    pub async fn generate_graph(&self, cli: &Cli) -> Result<()> {
        // getting last word
        let last_word = WordGetter::get_last_found_word(&cli.word_history)?;
        let word: String;
        if let Some(last) = last_word {
            if last.1 != Local::now().date_naive() {
                return Err(anyhow::anyhow!("Word of the day not found"));
            }
            word = last.0;
        } else {
            return Err(anyhow::anyhow!("Word of the day not found"));
        }

        // generating words to be tested and loading nearby words of day word => avoid duplication
        let best_word = Arc::new(Mutex::new(SolverStruct::default()));
        let words_words_list = WordGetter::get_words_of_all_found_words(
            &cli.words_directory,
            Some(&WordGetter::get_all_found_words_except(
                &cli.words_directory,
                &[&word.to_owned()],
            )?),
        )
        .await?;

        let size = words_words_list.iter().map(|v| v.len()).sum::<usize>();
        best_word.lock().await.words_data = HashSet::with_capacity(size + 1000);

        let mut words_list: HashSet<&String> = HashSet::with_capacity(size);

        let words_of_day_word =
            WordGetter::get_cemantix_words_of_found_word(&word, &cli.words_directory)?;
        best_word.lock().await.words_data.extend(words_of_day_word);

        let callback_best = |s: Arc<Mutex<SolverStruct>>, data: Vec<(String, Option<f32>)>| async move {
            let mut ss = s.lock().await;
            ss.words_data.extend(
                data.iter()
                    .filter(|v| v.1.is_some())
                    .map(|v| CemantixWord::new(v.0.to_owned(), 0, v.1.unwrap())),
            );
            Ok(false)
        };
        for words in words_words_list.iter() {
            words_list.extend(words.iter());
        }
        send_words(
            words_list,
            self.batch_size,
            best_word.clone(),
            callback_best,
        )
        .await;
        drop(words_words_list);
        let best = best_word.lock().await;
        let data = &best.words_data;
        let file = WordGetter::get_file_word(&word, false, true, false, &cli.words_directory)?;
        serde_json::to_writer(file, data)?;

        Ok(())
    }
}
