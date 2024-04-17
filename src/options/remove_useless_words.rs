use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use clap::Args;
use futures::{future::join_all, lock::Mutex};

use crate::utils::send_request;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Args)]
pub struct Ruw {
    /// source file
    pub source_filename: String,
    /// file destination
    pub destination_file: String,

    /// Line index from which remove useless words starts
    #[arg(short, long, default_value_t = 0)]
    pub starting_index: u32,

    /// Number of thread not over 200
    #[arg(short, long, default_value_t = 100)]
    pub batch_size: usize,
}
impl Ruw {
    pub async fn remove_useless_words(&mut self, verbose: bool) -> anyhow::Result<()> {
        if self.batch_size > 200 {
            println!("Set number of threads to 200");
            self.batch_size = 200;
        }
        let file = OpenOptions::new()
            .write(false)
            .read(true)
            .open(&self.source_filename)
            .map_err(|e| {
                anyhow::anyhow!("An error occured (file : {}) : {e}", self.source_filename)
            })?;
        let sorted_file: Arc<Mutex<fs::File>> = match OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&self.destination_file)
        {
            Ok(f) => Arc::new(Mutex::new(f)),
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "An error occured (file : {}) : {e}",
                    self.destination_file
                ));
            }
        };

        let mut words_list: Vec<String> = vec![String::new(); self.batch_size.to_owned()];
        let nb: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let mut total = 0;

        let reader = BufReader::new(file);
        for (_index, line) in reader.lines().enumerate() {
            if (_index as u32) < self.starting_index {
                continue;
            }
            total += 1;
            let word: String = line.unwrap();
            if _index % self.batch_size != self.batch_size - 1 {
                words_list[_index % self.batch_size] = word;
                continue;
            }
            words_list[_index % self.batch_size] = word.to_owned();
            let file_copy = Arc::clone(&sorted_file);
            let n = Arc::clone(&nb);
            self.launch_threads_ruw(words_list.clone(), file_copy, n, verbose)
                .await;
        }

        // there words left
        if total % self.batch_size != 0 {
            self.launch_threads_ruw(words_list, sorted_file.clone(), nb.clone(), verbose)
                .await;
        }

        println!("{:?} mots gardés sur {total} mots", nb);

        Ok(())
    }

    pub async fn launch_threads_ruw(
        &self,
        words_vec: Vec<String>,
        file: Arc<Mutex<fs::File>>,
        nb: Arc<AtomicUsize>,
        verbose: bool,
    ) {
        let mut futures = Vec::new();

        for word in words_vec.iter() {
            futures.push(send_request(&word));
        }

        let mut words_to_write: Vec<String> = Vec::new();

        let all_res = join_all(futures).await;
        for i in 0..self.batch_size {
            match all_res.get(i) {
                Some(v) => match v {
                    Ok(_) => {
                        words_to_write.push(words_vec.get(i).unwrap().to_string());
                    }
                    Err(e) => if e.to_string() == "unknown" {},
                },
                None => {}
            }
        }

        {
            let size = words_to_write.len();
            let _ = nb.fetch_add(size, Ordering::SeqCst); // += words_to_write.len();
        }

        if verbose {
            for word in words_to_write.iter() {
                println!("{} ajouté", word);
            }
        }

        let mut data: Vec<u8> = Vec::new();
        for word in words_to_write.iter() {
            data.extend(word.as_bytes());
            data.push(10);
        }
        {
            let mut f = file.lock().await;
            f.write_all(&data).unwrap();
            drop(f);
        }
    }
}
