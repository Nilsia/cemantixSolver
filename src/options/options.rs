use std::path::PathBuf;

use chrono::Local;
use clap::{command, Parser};

use super::{
    extend::Extend, graph::Graph, nearby::Nearby, remove_useless_words::Ruw, solve::Solve,
    sort::Sort,
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
        let start = Local::now();

        match &self.command {
            Commands::Solve(solve) => {
                match solve
                    .solve_cemantix(&solve.source_filename, solve.batch_size, &self)
                    .await
                {
                    Ok(_) => println!("Command solve executed successfully"),
                    Err(e) => eprintln!("ERROR : {e}"),
                }
            }
            Commands::Ruw(ruw) => {
                let mut ruw = ruw.clone();
                match ruw.remove_useless_words(self.verbose).await {
                    Ok(_) => println!("Command ruw (remove useless words) executed successfully"),
                    Err(e) => eprintln!("ERROR : {e}"),
                }
            }
            Commands::Nearby(nearby) => {
                match nearby.generate_nearby_word(&self.words_directory).await {
                    Ok(_) => println!("Command nearby executed successfully"),
                    Err(e) => eprintln!("ERROR : {e}"),
                }
            }

            Commands::Extend(extend) => match extend.extend_file(&self.words_directory).await {
                Ok(_) => {
                    println!("Command extend executed successfully");
                }
                Err(e) => eprintln!("ERROR : {e}"),
            },
            Commands::Sort(sort) => match sort.sort_file(&sort.source_file).await {
                Ok(_) => {
                    println!("Command sort executed successfully ! ");
                }
                Err(e) => {
                    eprintln!("ERROR : {e}");
                }
            },
            Commands::Graph(graph) => match graph.generate_graph(self, None).await {
                Ok(_) => println!("Semantix for nearby words executed"),
                Err(e) => eprintln!("Error: {e}"),
            },
        };
        let end = Local::now();
        let diff = end - start;
        println!("This operation took {}s ", diff.num_seconds());
    }
}
