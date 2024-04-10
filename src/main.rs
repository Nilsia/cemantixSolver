use anyhow::Result;
use cemantix_word::CemantixWord;
use reqwest::Response;
use std::iter::Extend;
use std::{
    collections::HashSet,
    fs::{self, read_dir, read_to_string, OpenOptions},
    io::{BufRead, BufReader, ErrorKind, Write},
    path::PathBuf,
    process::Command,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
const HISTORY_FORMAT: &str = "%d-%m-%Y";

pub mod cemantix_word;
mod options;

use chrono::{Local, NaiveDate};
use clap::Parser;
use futures::{future::join_all, lock::Mutex, Future};
use options::Cli;
use serde_json::Value;

struct SolverStruct {
    word: String,
    score: f32,
    filename: String,
    words_data: HashSet<CemantixWord>,
}

impl Default for SolverStruct {
    fn default() -> Self {
        Self {
            word: String::new(),
            score: 0.0,
            filename: String::new(),
            words_data: HashSet::new(),
        }
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut cli = options::Cli::parse();
    // env::set_var("RUST_BACKTRACE", "1");

    cli.matching().await;

    Ok(())
}

async fn sort_file(filename: &str) -> Result<()> {
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

async fn get_all_words(filename: &str, file_init: &mut Option<fs::File>) -> Result<Vec<String>> {
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

async fn get_words_of_all_found_words(
    words_fcontainer_name: &str,
    found_words: Option<&[String]>,
) -> Result<Vec<Vec<String>>> {
    let mut words_list: Vec<Vec<String>> = Vec::new();
    let mut f: Option<Vec<String>> = None;
    if found_words.is_none() {
        f = Some(get_all_found_word(words_fcontainer_name)?);
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
        words_list.push(get_words_of_found_word(word, words_fcontainer_name).await?);
    }

    Ok(words_list)
}

async fn get_words_of_found_word(word: &str, words_fcontainer_name: &str) -> Result<Vec<String>> {
    Ok(
        get_cemantix_words_of_found_word(word, words_fcontainer_name)?
            .into_iter()
            .map(|c| c.word.to_owned())
            .collect(),
    )
}

fn get_cemantix_words_of_found_word(
    word: &str,
    words_fcontainer_name: &str,
) -> Result<Vec<CemantixWord>> {
    let file_content = read_to_string(PathBuf::from(words_fcontainer_name).join(word))?;
    Ok(serde_json::from_str::<Vec<CemantixWord>>(&file_content)?)
}

fn get_all_found_words_except(
    words_fcontainer_name: &str,
    words_exception: &[&String],
) -> Result<Vec<String>> {
    let mut data = get_all_found_word(words_fcontainer_name)?;
    data.retain(|v| !words_exception.contains(&v));
    Ok(data)
}

async fn extend_file(filename: &str, words_fcontainer_name: &str) -> Result<()> {
    let mut words_to_add: Vec<String> = Vec::new();

    let mut file = None;
    let words = get_all_words(filename, &mut file).await?;

    for v in get_words_of_all_found_words(words_fcontainer_name, None)
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

async fn adding_word_to_historic(
    word: &str,
    word_history_filename: &str,
    words_fcontainer_name: &str,
) -> Result<()> {
    // check if the word has already been found (file exists, so file is returned)
    if let Ok(_) = get_file_word(word, false, true, false, words_fcontainer_name) {
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
    let mut data_to_write: Vec<u8> =
        (word.to_owned() + &" : ".to_string() + &Local::now().format(HISTORY_FORMAT).to_string())
            .as_bytes()
            .to_vec();
    data_to_write.push(10);
    file.write_all(&data_to_write)?;

    Ok(())
}

/// returns the file which stores the closest word of a found word
fn get_file_word(
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

async fn generate_nearby_word(word: &str, words_dir: &str) -> Result<()> {
    let file_content = get_nearby(&word).await?;
    if file_content.is_empty() {
        return Err(anyhow::anyhow!(format!(
            "Impossible de récupérer les mots proches de {word}"
        )));
    }
    let mut file_word = match get_file_word(word, true, true, false, words_dir) {
        Ok(f) => f,
        Err(e) => {
            match e.kind() {
                ErrorKind::NotFound => {
                    eprintln!("An error occured : {e}");
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

    if let Err(_) = file_word.write(&file_content.as_bytes()) {
        eprintln!("Error cannot write data to file '{word}'");
    } else {
        println!("Successfully writen data into file '{word}'");
    }

    Ok(())
}

async fn solve_cemantix(filename: &str, batch_size: usize, cli: &Cli) -> Result<()> {
    let last_word = get_last_found_word(&cli.word_history)?;
    if let Some(last) = last_word {
        if last.1 == Local::now().date_naive() {
            println!("Word already found ({}) !", last.0);
            return Ok(());
        }
    }
    let file = OpenOptions::new().read(true).open(filename)?;

    let reader = BufReader::new(file);

    let best_word = Arc::new(Mutex::new(SolverStruct::default()));
    let callback_solver = |best_word: Arc<Mutex<SolverStruct>>,
                           data: Vec<(String, Option<f32>)>| async move {
        let winner = data
            .iter()
            .filter(|v| v.1.is_some())
            .max_by(|x, y| x.1.unwrap().total_cmp(y.1.as_ref().unwrap()));
        if let Some(winner) = winner {
            let value = winner.1.unwrap();
            if value == 1.0 {
                let mut best_w = best_word.lock().await;
                best_w.score = value;
                best_w.word = winner.0.to_owned();
                println!("word found : {} ", winner.0);
                return Ok(true);
            } else {
                let mut best_w = best_word.lock().await;
                if value > best_w.score {
                    best_w.score = value;
                    best_w.word = winner.0.to_owned();
                    drop(best_w);
                    println!("New best word : {} with a score of {}", winner.0, value);
                }
            }
        }
        Ok(false)
    };
    send_words(
        reader.lines().flatten(),
        batch_size,
        best_word.clone(),
        callback_solver,
    )
    .await;

    // save new found word and new words related to found word
    let b = best_word.lock().await;
    if let Err(e) = adding_word_to_historic(&b.word, &cli.word_history, &cli.words_directory).await
    {
        eprintln!("Cannot append {} to historical words : {e}", b.word);
    }
    if let Err(e) = extend_file(&b.filename, &cli.words_directory).await {
        eprintln!("Cannot extend file {} : {e}", b.filename);
    }
    match generate_nearby_word(&b.word, &cli.words_directory).await {
        Ok(_) => {
            println!("Nearby words generated")
        }
        Err(_) => {}
    }

    match &cli.command {
        options::Commands::Solve(v) => {
            if v.graph {
                generate_graph(batch_size, cli).await?
            }
        }
        _ => (),
    }
    Ok(())
}

async fn generate_graph(batch_size: usize, cli: &Cli) -> Result<()> {
    // getting last word
    let last_word = get_last_found_word(&cli.word_history)?;
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
    let words_words_list = get_words_of_all_found_words(
        &cli.words_directory,
        Some(&get_all_found_words_except(
            &cli.words_directory,
            &[&word.to_owned()],
        )?),
    )
    .await?;

    let size = words_words_list.iter().map(|v| v.len()).sum::<usize>();
    best_word.lock().await.words_data = HashSet::with_capacity(size + 1000);

    let mut words_list: HashSet<&String> = HashSet::with_capacity(size);

    let words_of_day_word = get_cemantix_words_of_found_word(&word, &cli.words_directory)?;
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
    send_words(words_list, batch_size, best_word.clone(), callback_best).await;
    drop(words_words_list);
    let best = best_word.lock().await;
    let data = &best.words_data;
    let file = get_file_word(&word, false, true, false, &cli.words_directory)?;
    serde_json::to_writer(file, data)?;

    Ok(())
}

async fn send_words<T, F>(
    reader: T,
    nb_thread: usize,
    best_word: Arc<Mutex<SolverStruct>>,
    callback: impl Fn(Arc<Mutex<SolverStruct>>, Vec<(String, Option<f32>)>) -> F,
) where
    T: IntoIterator,
    T::Item: std::string::ToString,
    F: Future<Output = Result<bool>>,
{
    let mut words_list: Vec<String> = vec![String::new(); nb_thread];

    let mut iterator = reader.into_iter();
    let mut taken = iterator.by_ref().take(nb_thread);

    loop {
        words_list.clear();
        words_list = taken.map(|v| v.to_string()).collect::<Vec<String>>();
        // words_list.sort();
        // words_list.dedup();
        if words_list.len() == 0 {
            break;
        }
        let data = launch_threads_solve(words_list.clone()).await;
        if let Ok(v) = callback(best_word.clone(), data).await {
            if v {
                break;
            }
        };
        taken = iterator.by_ref().take(nb_thread);
    }
}

fn get_all_found_word(words_fcontainer_name: &str) -> Result<Vec<String>> {
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

async fn launch_threads_solve(words_batch: Vec<String>) -> Vec<(String, Option<f32>)> {
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

async fn remove_useless_words(
    source_filename: &str,
    destination_filename: &str,
    verbose: &bool,
    stating_index: &u32,
    vec_size: &usize,
) -> Result<()> {
    let file = match OpenOptions::new()
        .write(false)
        .read(true)
        .open(source_filename)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("An error occured (file : {}) : {e}", source_filename);
            return Err(anyhow::anyhow!(e));
        }
    };
    let sorted_file: Arc<Mutex<fs::File>> = match OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(destination_filename)
    {
        Ok(f) => Arc::new(Mutex::new(f)),
        Err(e) => {
            eprintln!("An error occured (file : {}) : {e}", destination_filename);
            return Err(anyhow::anyhow!(e));
        }
    };

    let mut words_list: Vec<String> = vec![String::new(); *vec_size];
    let nb: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let mut total = 0;

    let reader = BufReader::new(file);
    for (_index, line) in reader.lines().enumerate() {
        if &(_index as u32) < stating_index {
            continue;
        }
        total += 1;
        let word: String = line.unwrap();
        if _index % vec_size != vec_size - 1 {
            words_list[_index % vec_size] = word;
            continue;
        }
        words_list[_index % vec_size] = word.to_owned();
        let file_copy = Arc::clone(&sorted_file);
        let n = Arc::clone(&nb);
        launch_threads_ruw(words_list.clone(), file_copy, n, verbose, vec_size).await;
    }

    // there words left
    if total % vec_size != 0 {
        launch_threads_ruw(
            words_list,
            sorted_file.clone(),
            nb.clone(),
            verbose,
            &(total % *vec_size),
        )
        .await;
    }

    println!("{:?} mots gardés sur {total} mots", nb);

    Ok(())
}

async fn launch_threads_ruw(
    words_vec: Vec<String>,
    file: Arc<Mutex<fs::File>>,
    nb: Arc<AtomicUsize>,
    verbose: &bool,
    nb_words: &usize,
) {
    let mut futures = Vec::new();

    for word in words_vec.iter() {
        futures.push(send_request(&word));
    }

    let mut words_to_write: Vec<String> = Vec::new();

    let all_res = join_all(futures).await;
    for i in 0..*nb_words {
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

    if *verbose {
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

async fn get_nearby(word: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let params = [("word", word)];

    let a = client
        .post("https://cemantix.certitudes.org/nearby")
        .form(&params)
        .header("Content-type", "application/x-www-form-urlencoded");

    Ok(a.send().await?.text().await?)
}

fn generate_client(params: &[(&str, &str)]) -> reqwest::RequestBuilder {
    let client = reqwest::Client::new();
    client
        .post("https://cemantix.certitudes.org/score")
        .form(&params)
        .header("Content-type", "application/x-www-form-urlencoded")
        .header("Origin", "https://cemantix.certitudes.org")
}

async fn send_request(word: &str) -> Result<f32> {
    let params = [("word", word)];
    let mut a = generate_client(&params);
    let mut i = 0;
    let mut response: Result<Response, reqwest::Error> = a.send().await;
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
                    eprintln!("Ouille");
                    return Err(anyhow::anyhow!(format!("Unexpected error : {e}")));
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

fn get_last_found_word(word_history_filename: &str) -> Result<Option<(String, NaiveDate)>> {
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
                .map(|v| NaiveDate::parse_from_str(v.trim(), HISTORY_FORMAT));
            if let (Some(w), Some(d)) = (word, date) {
                Ok(Some((w, d?)))
            } else {
                Err(anyhow::anyhow!("Invalid end of file"))
            }
        }
        None => Ok(None),
    }
}
