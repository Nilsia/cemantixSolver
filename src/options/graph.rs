use std::{collections::HashSet, sync::Arc};

use anyhow::Result;
use chrono::Local;
use clap::Args;
use futures::lock::Mutex;

use crate::{cemantix_word::CemantixWord, utils::send_words, words_getter::WordGetter};

use super::{options::Cli, solve::DataThread};

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

    pub async fn generate_graph(
        &self,
        cli: &Cli,
        calculated_data: Option<Arc<Mutex<DataThread>>>,
    ) -> Result<()> {
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

        let words_words_list = WordGetter::get_words_of_all_found_words(
            &cli.words_directory,
            Some(&WordGetter::get_all_found_words_except(
                &cli.words_directory,
                &[&word.to_owned()],
            )?),
        )
        .await?;
        let large_words_number = words_words_list.iter().map(|v| v.len()).sum::<usize>();

        // generating words to be tested and loading nearby words of day word => avoid duplication
        let (best_word, large_words_number) = if let Some(b) = calculated_data {
            let len = b.lock().await.words_data.len();
            (b, len + large_words_number)
        } else {
            let b = Arc::new(Mutex::new(DataThread::new(
                word.to_owned(),
                1.0,
                HashSet::new(),
                0,
                cli.word_history.to_owned(),
            )));
            b.lock().await.words_data = HashSet::with_capacity(large_words_number + 1000);
            (b, large_words_number)
        };
        if best_word.lock().await.word != word {
            return Err(anyhow::anyhow!(
                "Error: {} (given through arguments) != {} (given from previous calculation)",
                word,
                best_word.lock().await.word
            ));
        }

        let mut words_list: HashSet<&String> = HashSet::with_capacity(large_words_number);

        let words_of_day_word =
            WordGetter::get_cemantix_words_of_found_word(&word, &cli.words_directory)?;
        best_word.lock().await.words_data.extend(words_of_day_word);

        let callback_best = |s: Arc<Mutex<DataThread>>, data: Vec<(String, Option<f32>)>| async move {
            let mut ss = s.lock().await;
            ss.words_data.extend(
                data.iter()
                    .filter(|v| v.1.is_some())
                    .map(|v| CemantixWord::new(v.0.to_owned(), 0, v.1.unwrap())),
            );
            Ok(false)
        };

        // creating HashSet that has to be iterated
        for words in words_words_list.iter() {
            words_list.extend(words.iter());
        }
        let b = best_word.lock().await;
        // removing all words previously calculated
        words_list.retain(|cw| !b.words_data.iter().any(|cw_wd| &&cw_wd.word == cw));
        let reduced_words_number = words_list.len();
        send_words(
            words_list,
            self.batch_size,
            best_word.clone(),
            callback_best,
            cli.verbose,
        )
        .await;
        println!(
            "{} words have been tested and added to the file {} !",
            reduced_words_number, word
        );
        drop(words_words_list);
        best_word.lock().await.save_into_file(cli)?;

        Ok(())
    }
}
