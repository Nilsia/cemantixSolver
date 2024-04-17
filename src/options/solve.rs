use crate::options::extend::Extend;
use crate::options::options::LogLevel;
use anyhow::Result;
use chrono::Local;
use clap::Args;
use futures::future::join_all;
use futures::lock::Mutex;
use std::collections::HashSet;
use std::io::BufRead;
use std::{fs::OpenOptions, io::BufReader, sync::Arc};

use crate::utils::{adding_word_to_historic, send_request, send_words};
use crate::{cemantix_word::CemantixWord, words_getter::WordGetter};

use super::graph::Graph;
use super::nearby::Nearby;
use super::options::Cli;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Args)]
pub struct Solve {
    // Source file of the words to brute force
    pub source_filename: String,

    /// Line index from which solving starts
    #[arg(short, long, default_value_t = 0)]
    pub starting_index: u32,

    /// Number of words in batches not over 200
    #[arg(short, long, default_value_t = 100)]
    pub batch_size: usize,

    /// fetch data for graph generation
    #[arg(short, long, default_value_t = false)]
    pub graph: bool,
}

#[derive(Debug)]
pub struct DataThread {
    pub word: String,
    score: f32,
    filename: String,
    pub words_data: HashSet<CemantixWord>,
    pub nb_tested_words: usize,
}

impl DataThread {
    pub fn new(
        word: String,
        score: f32,
        words_data: HashSet<CemantixWord>,
        nb_tested_words: usize,
        words_fcontainer: String,
    ) -> Self {
        Self {
            filename: words_fcontainer + &word,
            word,
            score,
            words_data,
            nb_tested_words,
        }
    }

    pub fn save_into_file(&mut self, cli: &Cli) -> Result<()> {
        let file = WordGetter::get_file_word(&self.word, false, true, false, &cli.words_directory)?;
        let mut copy = Vec::from_iter(self.words_data.iter());
        copy.sort();
        Ok(serde_json::to_writer(file, &copy)?)
    }
}

impl Default for DataThread {
    fn default() -> Self {
        Self {
            word: String::new(),
            score: 0.0,
            filename: String::new(),
            words_data: HashSet::new(),
            nb_tested_words: 0,
        }
    }
}

impl Solve {
    pub async fn solve_cemantix(&self, filename: &str, batch_size: usize, cli: &Cli) -> Result<()> {
        let last_word = WordGetter::get_last_found_word(&cli.word_history)?;
        if let Some(last) = last_word {
            if last.1 == Local::now().date_naive() {
                cli.log_and_print(
                    &format!("Word already found ({}) !", last.0),
                    LogLevel::Warn,
                )?;
                return Ok(());
            }
        }

        let reader = BufReader::new(OpenOptions::new().read(true).open(filename)?);
        let reader2 = BufReader::new(OpenOptions::new().read(true).open(filename)?);

        let best_word = Arc::new(Mutex::new(DataThread::default()));
        let callback_solver = |best_word: Arc<Mutex<DataThread>>,
                               data: Vec<(String, Option<f32>)>| async move {
            //TODO use new fucnton fromTuple
            let winner = data
                .iter()
                .filter(|v| v.1.is_some())
                .max_by(|x, y| x.1.unwrap().total_cmp(y.1.as_ref().unwrap()));
            if let Some(winner) = winner {
                let value = winner.1.unwrap();
                let mut best_w = best_word.lock().await;
                best_w.nb_tested_words += data.len();
                if value == 1.0 {
                    best_w.score = value;
                    best_w.word = winner.0.to_owned();
                    cli.log_and_print(&format!("word found : {} ", winner.0), LogLevel::Info)?;
                    return Ok(true);
                } else {
                    if value > best_w.score {
                        best_w.score = value;
                        best_w.word = winner.0.to_owned();
                        println!("New best word : {} with a score of {}", winner.0, value);
                    }
                }
                drop(best_w);
            }
            Ok(false)
        };
        send_words(
            reader.lines().flatten().count(),
            reader2.lines().flatten(),
            batch_size,
            best_word.clone(),
            callback_solver,
            cli.verbose,
        )
        .await;

        let b = best_word.lock().await;
        cli.log_and_print(
            &format!("{} words have been tested !", b.nb_tested_words),
            LogLevel::Info,
        )?;

        // save new found word and new words related to found word
        if let Err(e) = adding_word_to_historic(&b.word, &cli.word_history, cli).await {
            cli.log_and_print(
                &format!("Cannot append {} to historical words : {e}", b.word),
                LogLevel::Error,
            )?;
        }
        if let Err(e) = Extend::new(b.filename.to_owned())
            .extend_file(&cli.words_directory)
            .await
        {
            cli.log_and_print(
                &format!("Cannot extend file {} : {e}", b.filename),
                LogLevel::Error,
            )?;
        }
        if Nearby::new(b.word.to_owned())
            .generate_nearby_word(&cli.words_directory, cli)
            .await
            .is_ok()
        {
            println!("Nearby words generated");
        }
        drop(b);

        // TODO avoid resending same data
        if self.graph {
            cli.log_and_print("Generating graph", LogLevel::Info)?;
            Graph::new(self.batch_size)
                .generate_graph(cli, Some(best_word.clone()))
                .await?
        }
        Ok(())
    }
    pub async fn launch_threads_solve(words_batch: Vec<String>) -> Vec<(String, Option<f32>)> {
        join_all(words_batch.iter().map(|word| send_request(&word)))
            .await
            .iter()
            .enumerate()
            .map(|(i, v)| {
                (
                    words_batch.get(i).unwrap().to_owned(),
                    v.as_ref().ok().cloned(),
                )
            })
            .collect::<Vec<(String, Option<f32>)>>()
    }
}
