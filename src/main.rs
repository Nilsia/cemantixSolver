use std::{
    error::Error,
    fs::{self, read_dir, read_to_string, OpenOptions},
    io::{BufRead, BufReader, ErrorKind, Write},
    path::PathBuf,
    process::Command,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

mod options;

use chrono::Local;
use clap::Parser;
use futures::{future::join_all, lock::Mutex};
use options::Cli;
use serde_json::Value;

struct SolverStruct {
    word: String,
    score: f32,
    found: bool,
    filename: PathBuf,
}

impl SolverStruct {
    fn new(word: String, score: f32, filename: PathBuf) -> SolverStruct {
        SolverStruct {
            word,
            score,
            found: false,
            filename,
        }
    }

    fn f_to_string(&self) -> String {
        self.filename.as_os_str().to_str().unwrap().to_string()
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut cli = options::Cli::parse();
    //println!("{:#?} ", cli);

    cli.matching().await;

    Ok(())
}

async fn sort_file(filename: &PathBuf) -> Result<(), Box<dyn Error>> {
    //let mut file: Option<fs::File> = None;
    //let words = get_all_words(filename, &mut file).await?;

    let output = Command::new("sort").args([filename]).output()?.stdout;

    let mut i = 1;
    let mut file_exists = true;
    let mut new_filename = String::from("");

    while file_exists {
        new_filename = filename.to_str().unwrap().to_string() + &i.to_string();
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

async fn get_all_words(
    filename: &PathBuf,
    file_init: &mut Option<fs::File>,
) -> Result<Vec<String>, Box<dyn Error>> {
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
    words_fcontainer_name: &PathBuf,
) -> Result<Vec<Vec<String>>, Box<dyn Error>> {
    let folder_container = read_dir(words_fcontainer_name)?;
    let words: Vec<String> = folder_container
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
        .collect::<Vec<String>>();

    let mut words_list: Vec<Vec<String>> = Vec::new();

    for word in words.iter() {
        words_list.push(get_words_of_found_word(word, None, words_fcontainer_name).await?);
    }

    Ok(words_list)
}

async fn get_words_of_found_word(
    word: &String,
    _file: Option<std::fs::File>,
    words_fcontainer_name: &PathBuf,
) -> Result<Vec<String>, Box<dyn Error>> {
    let file_content =
        match read_to_string(words_fcontainer_name.to_str().unwrap().to_owned() + word.as_str()) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error cannot open file : {e}");
                return Err(Box::from(e));
            }
        };

    let j: Value = serde_json::from_str(&file_content)?;

    let mut words: Vec<String> = Vec::new();
    for items in j.as_array().iter() {
        for item in items.iter() {
            words.push(
                item.get(0)
                    .unwrap_or(&Value::String("".to_string()))
                    .to_string()
                    .replace("\"", ""),
            );
        }
    }

    Ok(words)
}

async fn extend_file(
    filename: &PathBuf,
    words_fcontainer_name: &PathBuf,
) -> Result<(), Box<dyn Error>> {
    let mut words_to_add: Vec<String> = Vec::new();

    let mut file = None;
    let words = get_all_words(filename, &mut file).await?;

    for v in get_words_of_all_found_words(words_fcontainer_name)
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
    word: &String,
    word_history_filename: &PathBuf,
    words_fcontainer_name: &PathBuf,
) -> Result<(), Box<dyn Error>> {
    // check if the word has already been found (file exists, so file is returned)
    if let Ok(_) = get_file_word(word, false, words_fcontainer_name).await {
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
            eprintln!(
                "Cannot open {}",
                word_history_filename.to_str().unwrap().to_string()
            );
            return Err(Box::from(e));
        }
    };
    let mut data_to_write: Vec<u8> =
        (word.to_owned() + &" : ".to_string() + &Local::now().format("%d-%m-%Y").to_string())
            .as_bytes()
            .to_vec();
    data_to_write.push(10);
    file.write_all(&data_to_write)?;

    Ok(())
}

async fn get_file_word(
    word: &String,
    create_new: bool,
    words_fcontainer_name: &PathBuf,
) -> Result<std::fs::File, std::io::Error> {
    // folder container does not exist
    if let Err(_) = fs::read_dir(words_fcontainer_name) {
        fs::create_dir(words_fcontainer_name)?;
    }

    let file = OpenOptions::new()
        .create_new(create_new)
        .write(true)
        .read(true)
        .open(words_fcontainer_name.to_str().unwrap().to_string() + &word.clone())?;
    return Ok(file);
}

async fn generate_nearby_word(word: &String, words_dir: &PathBuf) -> Result<(), Box<dyn Error>> {
    let file_content = get_nearby(&word).await;
    println!("wordsdir = {:?}", words_dir);
    if file_content.is_empty() {
        return Err(Box::from(format!(
            "Impossible de récupérer les mots proches de {word}"
        )));
    }
    let mut file_word = match get_file_word(word, true, words_dir).await {
        Ok(f) => f,
        Err(e) => {
            match e.kind() {
                ErrorKind::NotFound => {
                    eprintln!("An error occured : {e}");
                    return Err(Box::from(e));
                },
                ErrorKind::AlreadyExists => {
                    return Err(Box::from("Already generated"));
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

async fn solve_cemantix(
    filename: &PathBuf,
    vec_size: &usize,
    cli: &Cli,
) -> Result<(), Box<dyn Error>> {
    let file = OpenOptions::new().read(true).open(filename).unwrap();

    let reader = BufReader::new(file);

    let best_word = Arc::new(Mutex::new(SolverStruct::new(
        "".to_string(),
        0.0,
        filename.clone(),
    )));
    let mut words_list: Vec<String> = vec![String::new(); *vec_size];

    let mut words_tested: Vec<String> = Vec::new();

    for (_index, line) in reader.lines().enumerate() {
        let word: String = line.unwrap();
        //println!("{_index} : {word}");
        if words_tested.contains(&word) {
            continue;
        }
        words_tested.push(word.to_owned());
        if _index % vec_size != vec_size - 1 {
            words_list[_index % vec_size] = word;
            continue;
        }
        {
            let b = best_word.lock().await;
            if b.found {
                drop(b);
                break;
            }
            drop(b);
        }
        words_list[_index % vec_size] = word.to_owned();

        launch_threads_solve(
            words_list.clone(),
            best_word.clone(),
            vec_size,
            cli.words_directory.as_ref().unwrap(),
            cli.word_history.as_ref().unwrap(),
        )
        .await;
    }

    Ok(())
}

async fn launch_threads_solve(
    words_vec: Vec<String>,
    solver_struct: Arc<Mutex<SolverStruct>>,
    vec_size: &usize,
    words_fcontainer_name: &PathBuf,
    word_history_filename: &PathBuf,
) {
    let mut futures = Vec::new();

    for word in words_vec.iter() {
        futures.push(send_request(word.to_string()));
    }

    let res_all = join_all(futures).await;

    for i in 0..*vec_size {
        match res_all.get(i) {
            Some(v) => match v {
                Ok(f) => {
                    //println!("f = {f}");
                    let word = match words_vec.get(i) {
                        Some(d) => d,
                        None => {
                            panic!("Unexpected value !! (None)");
                        }
                    };
                    let mut b = solver_struct.lock().await;
                    if f > &b.score {
                        println!("Nouveau mot : {word} avec un score de {f}");
                        b.score = f.clone();
                        b.word = word.clone();
                    }
                    if b.score == 1.0 && !b.found {
                        b.found = true;
                        println!("Mot trouvé : {}", word);
                        if let Err(e) = adding_word_to_historic(
                            &word,
                            word_history_filename,
                            words_fcontainer_name,
                        )
                        .await
                        {
                            eprintln!("Cannot append {word} to historical words : {e}");
                        }
                        if let Err(e) = extend_file(&b.filename, words_fcontainer_name).await {
                            eprintln!("Cannot extend file {} : {e}", b.f_to_string());
                        }
                        //return
                        match generate_nearby_word(&word, words_fcontainer_name).await {
                            Ok(_) => {
                                println!("Nearby words generated")
                            }
                            Err(_) => {}
                        }
                    }
                    drop(b);
                }
                Err(e) => {
                    if e.to_string() == "unknown" {
                    } else {
                        eprintln!("An error occured keeping continue !!")
                    }
                }
            },
            None => {}
        }
    }
}

async fn remove_useless_words(
    source_filename: &PathBuf,
    destination_filename: &PathBuf,
    verbose: &bool,
    stating_index: &u32,
    vec_size: &usize,
) -> Result<(), Box<dyn Error>> {
    let file = match OpenOptions::new()
        .write(false)
        .read(true)
        .open(source_filename)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "An error occured (file : {}) : {e}",
                source_filename.to_str().unwrap().to_string()
            );
            return Err(Box::from(e));
        }
    };
    let sorted_file: Arc<Mutex<fs::File>> = match OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(destination_filename)
    {
        Ok(f) => Arc::new(Mutex::new(f)),
        Err(e) => {
            eprintln!(
                "An error occured (file : {}) : {e}",
                destination_filename
                    .as_os_str()
                    .to_str()
                    .unwrap()
                    .to_string()
            );
            return Err(Box::from(e));
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
        futures.push(send_request(word.to_string()));
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

async fn get_nearby(word: &String) -> String {
    let client = reqwest::Client::new();
    let params = [("word", word)];

    let a = client
        .post("https://cemantix.certitudes.org/nearby")
        .form(&params)
        .header("Content-type", "application/x-www-form-urlencoded");

    return a.send().await.unwrap().text().await.unwrap();
}

async fn send_request(word: String) -> Result<f32, Box<dyn Error>> {
    //println!("youi");
    let client = reqwest::Client::new();
    let params = [("word", word)];
    //println!("bof");
    let a = client
        .post("https://cemantix.certitudes.org/score")
        .form(&params)
        .header("Content-type", "application/x-www-form-urlencoded");
    //println!("dskfjhksdhfk");
    let response = match a.send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Une erreur : {e}");
            return Err(Box::from(e));
        }
    };
    //println!("popop");
    let json_parsed: Value = match response.status() {
        reqwest::StatusCode::OK => match response.text().await {
            Ok(text) => match serde_json::from_str(&text.as_str()) {
                Ok(parsed) => parsed,
                Err(_) => {
                    eprintln!("Error: cannot parse json");
                    return Err(Box::from("Unable to deserialize json"));
                }
            },
            Err(_) => {
                eprintln!("Error: cannot get text");
                return Err(Box::from("Unable get text"));
            }
        },
        reqwest::StatusCode::UNAUTHORIZED => {
            eprintln!("Unauthorized");
            return Err(Box::from("Unauthorized"));
        }
        e => {
            eprintln!("Ouille");
            return Err(Box::from(format!("Unexpected error : {e}")));
        }
    };
    //println!("insied");

    match json_parsed.get("error") {
        Some(_) => {
            //println!("Mot inconnu");
            return Err(Box::from("unknown"));
        }
        None => (),
    }

    let value = json_parsed.get("score");
    if value.is_some() {
        //println!("kdjsfjk = {}", value.unwrap().to_string());
        return Ok(value.unwrap().to_string().parse().unwrap());
    } else {
        return Err(Box::from("None value"));
    }
}
