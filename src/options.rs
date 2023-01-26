use std::path::PathBuf;

use clap::{command, Args, Parser};

use crate::{extend_file, generate_nearby_word, remove_useless_words, solve_cemantix, sort_file};
const DEFAULT_HISTORY_FILENAME: &str = "words_history";
const DEFAULT_WORDS_FOLDER: &str = "words_folder/";

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, clap::Subcommand, Debug)]
pub enum Commands {
    /// Find out the word of the day
    Solve(Solve),
    /// Remove all useless words of a file
    Ruw(Ruw),
    /// Generate the closest words of the word of the day
    Nearby(Nearby),
    /// Extend your file from the closest words of the word of the day
    Extend(Extend),
    /// Sort your file (A->Z)
    Sort(Sort),
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Args)]
pub struct Solve {
    // Source file of the words to brute force
    pub source_filename: PathBuf,

    /// Line index from which solving starts
    #[arg(short, long, default_value_t = 0)]
    pub starting_index: u32,

    /// Number of thread not over 200
    #[arg(short, long, default_value_t = 100)]
    pub nb_thread: usize,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Args)]
pub struct Ruw {
    /// source file
    pub source_filename: PathBuf,
    /// file destination
    pub destination_file: PathBuf,

    /// Line index from which remove useless words starts
    #[arg(short, long, default_value_t = 0)]
    pub starting_index: u32,

    /// Number of thread not over 200
    #[arg(short, long, default_value_t = 100)]
    pub nb_thread: usize,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Args)]
pub struct Nearby {
    /// The word of the day only
    pub word: String,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Args)]
pub struct Extend {
    /// Source file
    pub source_file: PathBuf,
}
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Args)]
pub struct Sort {
    /// source file
    pub source_file: PathBuf,
}

#[derive(Parser, Debug)]
#[command(version = "1.0")]
#[command(propagate_version = true)]
#[command(author, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// verbose mode
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,

    /// the directory that contains all the words found (nearby) [DEFAULT: ./words_folder]
    #[arg(long)]
    pub words_directory: Option<PathBuf>,

    /// declare the name of the file in which the history of words found will be written [DEFAULT: ./words_history]
    #[arg(long)]
    pub word_history: Option<PathBuf>,

    /// specify the current directory where the file will be added/written [DEFAULT: current directory]
    /// if --words-directory AND/OR --word-history specified, files with the same name will be created the working folder
    #[arg(long)]
    pub working_directory: Option<PathBuf>,
}

impl Cli {
    pub async fn matching(&mut self) {
        if self.working_directory.is_none() {
            let _ = self
                .working_directory
                .insert(self.generate_path(&vec!["./"]));

            if self.words_directory.is_none() {
                let _ = self.words_directory.insert(self.generate_path(&vec![
                    self.working_directory.as_ref().unwrap().to_str().unwrap_or("./"),
                    DEFAULT_WORDS_FOLDER,
                ]));
            } else {
                let _ = self.words_directory.insert(self.generate_path(&vec![
                    self.words_directory.as_ref().unwrap().to_str().unwrap(),
                    "/",
                ]));
            }

            if self.word_history.is_none() {
                let _ = self.word_history.insert(self.generate_path(&vec![
                    self.working_directory.as_ref().unwrap().to_str().unwrap_or("./"),
                    DEFAULT_HISTORY_FILENAME,
                ]));
            }
        } else {
            // working directory specified wo we have to see if we can generate files in it
            if self.words_directory.is_none() {
                let a = self.words_directory.insert(self.generate_path(&vec![
                    self.working_directory.as_ref().unwrap().to_str().unwrap(),
                    DEFAULT_WORDS_FOLDER,
                ]));
                println!("adding new {:?}", a);
            } else {
                let _ = self.words_directory.insert(self.generate_path(&vec![
                    self.working_directory.as_ref().unwrap().to_str().unwrap(),
                    self.words_directory.as_ref().unwrap().to_str().unwrap(),
                    "/",
                ]));
            }

            if self.word_history.is_none() {
                let _ = self.word_history.insert(self.generate_path(&vec![
                    self.working_directory.as_ref().unwrap().to_str().unwrap(),
                    DEFAULT_HISTORY_FILENAME,
                ]));
            } else {
                let _ = self.word_history.insert(self.generate_path(&vec![
                    self.working_directory.as_ref().unwrap().to_str().unwrap(),
                    self.word_history.as_ref().unwrap().to_str().unwrap(),
                ]));
            }
        }

        println!(
            "wkD : {:?}\nwdH : {:?}\nwdF : {:?}",
            self.working_directory, self.word_history, self.words_directory
        );

        match &self.command {
            Commands::Solve(name) => {
                match solve_cemantix(&name.source_filename, &name.nb_thread, &self).await {
                    Ok(_) => println!("Command solve executed successfully"),
                    Err(e) => eprintln!("ERROR : {e}"),
                }
            }
            Commands::Ruw(name) => {
                let mut nb_thread = name.nb_thread;
                if name.nb_thread > 200 {
                    println!("Set number of threads to 200");
                    nb_thread = 200;
                }
                match remove_useless_words(
                    &name.source_filename,
                    &name.destination_file,
                    &self.verbose,
                    &name.starting_index,
                    &nb_thread,
                )
                .await
                {
                    Ok(_) => println!("Command ruw (remove useless words) executed successfully"),
                    Err(e) => eprintln!("ERROR : {e}"),
                }
            }
            Commands::Nearby(name) => {
                match generate_nearby_word(&name.word, self.words_directory.as_ref().unwrap()).await
                {
                    Ok(_) => println!("Command nearby executed successfully"),
                    Err(e) => eprintln!("ERROR : {e}"),
                }
            }
            Commands::Extend(name) => {
                match extend_file(&name.source_file, self.words_directory.as_ref().unwrap()).await {
                    Ok(_) => {
                        println!("Command extend executed successfully");
                    }
                    Err(e) => eprintln!("ERROR : {e}"),
                }
            }
            Commands::Sort(name) => match sort_file(&name.source_file).await {
                Ok(_) => {
                    println!("Command sort executed successfully ! ");
                }
                Err(e) => {
                    eprintln!("ERROR : {e}");
                }
            },
        }
    }

    fn generate_path(&self, vec_str: &Vec<&str>) -> PathBuf {
        let mut path_puf = PathBuf::new();
        for ele in vec_str.iter() {
            path_puf.push(ele);
        }
        return path_puf;
    }
}
