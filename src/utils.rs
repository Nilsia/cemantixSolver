use anyhow::Result;
use chrono::Local;
use serde_json::Value;
use std::{fs::OpenOptions, io::Write, sync::Arc};

use futures::{lock::Mutex, Future};

use crate::{
    options::solve::{DataThread, Solve},
    words_getter::WordGetter,
};

pub async fn adding_word_to_historic(
    word: &str,
    word_history_filename: &str,
    words_fcontainer_name: &str,
) -> Result<()> {
    // check if the word has already been found (file exists, so file is returned)
    if let Ok(_) = WordGetter::get_file_word(word, false, true, false, words_fcontainer_name) {
        println!("Mot déjà trouvé inutile de l'enregistrer à nouveau");
        return Ok(());
    }

    // error: does not exist -> word not found / must not create it

    let mut file = match OpenOptions::new()
        .create(true)
        .append(true)
        .read(true)
        .open(word_history_filename)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Cannot open {}", word_history_filename);
            return Err(anyhow::anyhow!(e));
        }
    };
    let mut data_to_write: Vec<u8> = (word.to_owned()
        + &" : ".to_string()
        + &Local::now().format(crate::HISTORY_FORMAT).to_string())
        .as_bytes()
        .to_vec();
    data_to_write.push(10);
    file.write_all(&data_to_write)?;

    Ok(())
}
pub async fn send_words<T, F>(
    reader: T,
    batch_size: usize,
    best_word: Arc<Mutex<DataThread>>,
    callback: impl Fn(Arc<Mutex<DataThread>>, Vec<(String, Option<f32>)>) -> F,
    verbose: bool,
) where
    T: IntoIterator,
    T::Item: std::string::ToString,
    F: Future<Output = Result<bool>>,
{
    let mut words_list: Vec<String> = vec![String::new(); batch_size];
    let mut iterator = reader.into_iter();

    let itertator_len = iterator.by_ref().count();
    let mut count = 0;
    let mut last = 0;

    let mut taken = iterator.by_ref().take(batch_size);

    loop {
        words_list.clear();
        words_list = taken.map(|v| v.to_string()).collect::<Vec<String>>();
        // words_list.sort();
        // words_list.dedup();
        if words_list.len() == 0 {
            break;
        }
        let data = Solve::launch_threads_solve(words_list.clone()).await;
        count += batch_size;

        let tmp = ((count as f32) / 3.0).floor() as usize;
        if tmp != last {
            last = tmp;
            if verbose {
                println!("Current state : {count}/{itertator_len}");
            }
        }

        if let Ok(v) = callback(best_word.clone(), data).await {
            if v {
                break;
            }
        };
        taken = iterator.by_ref().take(batch_size);
    }
}
pub fn generate_client(params: &[(&str, &str)]) -> reqwest::RequestBuilder {
    let client = reqwest::Client::new();
    client
        .post("https://cemantix.certitudes.org/score")
        .form(&params)
        .header("Content-type", "application/x-www-form-urlencoded")
        .header("Origin", "https://cemantix.certitudes.org")
}

pub async fn send_request(word: &str) -> Result<f32> {
    let params = [("word", word)];
    let mut a = generate_client(&params);
    let mut i = 0;
    let mut response = a.send().await;
    while i < 5 && response.is_err() {
        i += 1;
        a = generate_client(&params);
        response = a.send().await;
    }
    match response {
        Ok(response) => {
            let json_parsed: Value = match response.status() {
                reqwest::StatusCode::OK => match response.text().await {
                    Ok(text) => match serde_json::from_str(&text.as_str()) {
                        Ok(parsed) => parsed,
                        Err(_) => {
                            eprintln!("Error: cannot parse json");
                            return Err(anyhow::anyhow!("Unable to deserialize json"));
                        }
                    },
                    Err(_) => {
                        eprintln!("Error: cannot get text");
                        return Err(anyhow::anyhow!("Unable get text"));
                    }
                },
                reqwest::StatusCode::UNAUTHORIZED => {
                    eprintln!("Unauthorized");
                    return Err(anyhow::anyhow!("Unauthorized"));
                }
                e => {
                    eprintln!("Unexpected error : {e}");
                    return Err(anyhow::anyhow!("Unexpected error : {e}"));
                }
            };
            match json_parsed.get("error") {
                Some(_) => {
                    return Err(anyhow::anyhow!("unknown"));
                }
                None => (),
            }

            let value = json_parsed.get("score");
            if value.is_some() {
                return Ok(value.unwrap().to_string().parse().unwrap());
            } else {
                return Err(anyhow::anyhow!("None value"));
            }
        }
        Err(_) => todo!(),
    }
}
