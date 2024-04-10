use std::path::PathBuf;

use clap::{command, Args, Parser};

use crate::{
    extend_file, generate_graph, generate_nearby_word, remove_useless_words, solve_cemantix,
    sort_file,
};
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
    /// Graph
    Graph(Graph),
}

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

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Args)]
pub struct Nearby {
    /// The word of the day only
    pub word: String,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Args)]
pub struct Extend {
    /// Source file
    pub source_file: String,
}
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Args)]
pub struct Sort {
    /// source file
    pub source_file: String,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Args)]
pub struct Graph {
    /// Number of words in batches not over 200
    #[arg(short, long, default_value_t = 100)]
    pub batch_size: usize,
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

    /// the directory that contains all the words found (nearby)
    #[arg(long, default_value_t = String::from(DEFAULT_WORDS_FOLDER))]
    pub words_directory: String,

    /// declare the name of the file in which the history of words found will be written
    #[arg(long, default_value_t = String::from(DEFAULT_HISTORY_FILENAME))]
    pub word_history: String,

    /// specify the current directory where the file will be added/written
    /// if --words-directory AND/OR --word-history specified, files with the same name will be created the working folder
    #[arg(long, default_value_t = String::from("./"))]
    pub working_directory: String,
}

impl Cli {
    pub async fn matching(&mut self) {
        let current = PathBuf::from(&self.working_directory);
        self.word_history = current.join(&self.word_history).display().to_string();
        self.words_directory = current.join(&self.words_directory).display().to_string();

        match &self.command {
            Commands::Solve(name) => {
                match solve_cemantix(&name.source_filename, name.batch_size, &self).await {
                    Ok(_) => println!("Command solve executed successfully"),
                    Err(e) => eprintln!("ERROR : {e}"),
                }
            }
            Commands::Ruw(name) => {
                let mut nb_thread = name.batch_size;
                if name.batch_size > 200 {
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
                match generate_nearby_word(&name.word, &self.words_directory).await {
                    Ok(_) => println!("Command nearby executed successfully"),
                    Err(e) => eprintln!("ERROR : {e}"),
                }
            }
            Commands::Extend(name) => {
                match extend_file(&name.source_file, &self.words_directory).await {
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
            Commands::Graph(g) => generate_graph(g.batch_size, self).await.unwrap(),
        }
    }
}
