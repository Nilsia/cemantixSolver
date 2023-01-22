use std::{
    error::Error,
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, ErrorKind, Write},
    process::Command,
};

use chrono::Local;
use serde_json::Value;

const FOLDER_CONTAINER_NAME: &str = "words_folder";

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    /*args :
        solve filename 3
        sort filename sorted_filename 4
        nearby word 3
    */
    if args.len() <= 1 {
        print_help();
    } else if args[1] == "solve" {
        if args.len() < 3 {
            eprintln!("nom du fichier manquant");
            return Ok(());
        }
        if let Err(e) = solve_cemantix(&args[2]).await {
            eprintln!("Error : {e}");
        }
    } else if args[1] == "removeuselesswords" || args[1] == "ruw" {
        if args.len() < 4 {
            eprintln!("Paramètre(s) manquant(s) pour le paramètres sort");
            return Ok(());
        }
        if let Err(e) = remove_useless_words(&args[2], &args[3]).await {
            eprintln!("Error : {e}");
        } else {
            println!("Fichier généré");
        }
    } else if args[1] == "nearby" && args.len() >= 3 {
        if args.len() < 3 {
            eprintln!("mot manquant");
            return Ok(());
        }
        if let Err(e) = generate_nearby_word(&args[2]).await {
            eprintln!("Error : {e}");
        }
    } else if args[1] == "extend" {
        if args.len() < 3 {
            println!("Paramètre manquant (nom du fichier)");
            return Ok(());
        }
        extend_file(&args[2]).await.unwrap();
    } else if args[1] == "sort" {
        if args.len() < 3 {
            println!("Paramètres manquant (nom du fichier)");
            return Ok(());
        }
        if let Err(e) = sort_file(&args[2]).await {
            eprintln!("An error occured : {e}");
        } else {
            println!("Fichier modifié avec succès, tous les mots ont été triés");
        }
    } else {
        print_help();
    }

    Ok(())
}

fn print_help() {
    println!("Paramètres possibles :");
    println!(" - solve [filename],");
    println!(" - removeuselesswords|ruw [filename] [sorted_filename],");
    println!(" - nearby [word],");
    println!(" - extend [filename],");
    println!(" - sort [filename]")
}

async fn sort_file(filename: &String) -> Result<(), Box<dyn Error>> {
    //let mut file: Option<fs::File> = None;
    //let words = get_all_words(filename, &mut file).await?;

    let output = Command::new("sort")
        .args([filename])
        .output()?.stdout;

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

async fn get_all_words(
    filename: &String,
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

async fn extend_file(filename: &String) -> Result<(), Box<dyn Error>> {
    let mut words_to_add: Vec<String> = Vec::new();

    let mut file = None;
    let words = get_all_words(filename, &mut file).await?;

    let folder_container = match fs::read_dir(FOLDER_CONTAINER_NAME) {
        Ok(d) => d,
        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                fs::create_dir(FOLDER_CONTAINER_NAME).unwrap();
                fs::read_dir(FOLDER_CONTAINER_NAME).unwrap()
            } else {
                return Err(Box::from(e));
            }
        }
    };

    for word_file in folder_container {
        let word_file = word_file?;
        let file_content = fs::read_to_string(word_file.path()).unwrap();
        let j: Value = serde_json::from_str(&file_content).unwrap();

        for items in j.as_array().iter() {
            for item in items.iter() {
                let word: String = item
                    .get(0)
                    .unwrap_or(&Value::String("".to_string()))
                    .to_string()
                    .replace("\"", "");
                //println!("word = {word}");
                if !word.is_empty()
                    && !words.contains(&word)
                    && item.get(2).is_some()
                    && !words_to_add.contains(&word)
                {
                    words_to_add.push(word.to_owned());
                }
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

async fn adding_word_to_historic(word: &String) -> Result<(), Box<dyn Error>> {
    let filename = "words_history";

    // check if the word has already been found (file exists)
    if let Err(_) = get_file_word(word, false).await {
        println!("Mot déjà trouvé inutile de l'enregistrer à nouveau");
        return Ok(());
    }

    let mut file = match OpenOptions::new()
        .create(true)
        .append(true)
        .read(true)
        .open(filename)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Cannot open {filename}");
            return Err(Box::from(e));
        }
    };
    let mut data_to_write: Vec<u8> = (word.to_owned()
        + &" -> ".to_string()
        + &Local::now().format("%Y-%m-%d %H:%M:%S").to_string())
        .as_bytes()
        .to_vec();
    data_to_write.push(10);
    file.write_all(&data_to_write)?;

    Ok(())
}

async fn get_file_word(word: &String, create_new: bool) -> Result<std::fs::File, Box<dyn Error>> {
    // folder container does not exist
    if let Err(_) = fs::read_dir(FOLDER_CONTAINER_NAME) {
        fs::create_dir(FOLDER_CONTAINER_NAME)?;
    }

    let file = match OpenOptions::new()
        .create_new(create_new)
        .write(true)
        .open(FOLDER_CONTAINER_NAME.to_string() + &word.clone())
    {
        Ok(file) => file,
        Err(e) => {
            return Err(Box::from(e));
        }
    };

    return Ok(file);
}

async fn generate_nearby_word(word: &String) -> Result<(), Box<dyn Error>> {
    let file_content = get_nearby(&word).await;
    if file_content.is_empty() {
        return Err(Box::from(format!(
            "Impossible de récupérer les mots proches de {word}"
        )));
    }
    let mut file_word = match get_file_word(word, true).await {
        Ok(f) => f,
        Err(e) => {
            eprintln!("An error occured : {e}");
            return Err(Box::from(e));
        }
    };

    if let Err(_) = file_word.write(&file_content.as_bytes()) {
        eprintln!("Error cannot write data to file '{word}'");
    } else {
        println!("Successfully writen data into file '{word}'");
    }

    Ok(())
}

async fn solve_cemantix(filename: &String) -> Result<(), Box<dyn Error>> {
    let file = OpenOptions::new().read(true).open(filename).unwrap();

    let reader = BufReader::new(file);
    let mut word = String::new();
    let mut score: f32 = 0.0;

    let mut words_tested: Vec<String> = Vec::new();

    for (_index, line) in reader.lines().enumerate() {
        let line: String = line.unwrap();

        if words_tested.contains(&line) {
            continue;
        }

        match send_request(line.to_owned()).await {
            Ok(value) => {
                words_tested.push(line.clone());
                if value > score {
                    println!("Nouveau mot : {line} avec un score de {value}");
                    score = value;
                    word = line;
                }
            }
            Err(e) => {
                if e.to_string().eq("unknown") {
                } else {
                    eprintln!("An error occured !! Keeping continue !!!");
                    continue;
                }
            }
        }
        if score == 1.0 {
            println!("Mot trouvé : {}", word);
            if let Err(e) = adding_word_to_historic(&word).await {
                eprintln!("Cannot append {word} to historical words : {e}");
            }
            if let Err(e) = extend_file(filename).await {
                eprintln!("Cannot extend file {filename} : {e}");
            }
            return generate_nearby_word(&word).await;
        }
    }

    Ok(())
}

async fn remove_useless_words(
    filename: &String,
    sorted_filename: &String,
) -> Result<(), Box<dyn Error>> {
    let file = match OpenOptions::new()
        .write(false)
        .read(true)
        .open("francais.txt")
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("An error occured (file : {filename}) : {e}");
            return Err(Box::from(e));
        }
    };
    let mut sorted_file = match OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(sorted_filename)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("An error occured (file : {sorted_filename}) : {e}");
            return Err(Box::from(e));
        }
    };

    let reader = BufReader::new(file);
    for (_index, line) in reader.lines().enumerate() {
        let line: String = line.unwrap();
        match send_request(line.to_owned()).await {
            Ok(_) => {
                let mut data = line.as_bytes().to_vec();
                data.push(10);
                sorted_file.write_all(&data).unwrap();
            }
            Err(e) => {
                if e.to_string().eq("unknown") {
                } else {
                    eprintln!("An error occured !! Need to abort");
                    return Ok(());
                }
            }
        }
    }

    Ok(())
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
    let client = reqwest::Client::new();
    let params = [("word", word)];
    let a = client
        .post("https://cemantix.certitudes.org/score")
        .form(&params)
        .header("Content-type", "application/x-www-form-urlencoded");
    let response = a.send().await?;
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

    match json_parsed.get("error") {
        Some(_) => {
            //println!("Mot inconnu");
            return Err(Box::from("unknown"));
        }
        None => (),
    }

    let value = json_parsed.get("score");

    if value.is_some() {
        return Ok(value.unwrap().to_string().parse().unwrap());
    } else {
        return Err(Box::from("None value"));
    }
}
