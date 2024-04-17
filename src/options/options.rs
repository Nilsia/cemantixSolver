use anyhow::Result;
use std::{fmt::Display, fs::OpenOptions, io::Write, path::PathBuf};

use chrono::Local;
use clap::{command, Parser};

use super::{
    extend::Extend, graph::Graph, nearby::Nearby, remove_useless_words::Ruw, solve::Solve,
    sort::Sort,
};

pub enum LogLevel {
    Warn,
    Error,
    Info,
}

impl ToString for LogLevel {
    fn to_string(&self) -> String {
        String::from(match self {
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Info => "INFO",
        })
    }
}

const DEFAULT_HISTORY_FILENAME: &str = "words_history";
const DEFAULT_WORDS_FOLDER: &str = "words_folder/";
const LOG_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

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

impl Display for Commands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Commands::Solve(_) => "Solve",
            Commands::Ruw(_) => "Remove useless words",
            Commands::Nearby(_) => "Nearby",
            Commands::Extend(_) => "Extend",
            Commands::Sort(_) => "Sort",
            Commands::Graph(_) => "Graph",
        })
    }
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

    #[arg(long, short)]
    pub log: Option<String>,
}

impl Cli {
    fn init(&mut self) {
        let current = PathBuf::from(&self.working_directory);
        self.word_history = current.join(&self.word_history).display().to_string();
        self.words_directory = current.join(&self.words_directory).display().to_string();
        if self.log.is_some() {
            let _ = self.log.insert(
                current
                    .join(self.log.as_ref().unwrap())
                    .display()
                    .to_string(),
            );
        }
    }
    pub async fn matching(&mut self) -> Result<()> {
        self.init();
        let start = Local::now();

        match &self.command {
            Commands::Solve(solve) => {
                solve
                    .solve_cemantix(&solve.source_filename, solve.batch_size, &self)
                    .await
            }
            Commands::Ruw(ruw) => {
                let mut ruw = ruw.clone();
                ruw.remove_useless_words(self.verbose).await
            }
            Commands::Nearby(nearby) => {
                nearby
                    .generate_nearby_word(&self.words_directory, self)
                    .await
            }

            Commands::Extend(extend) => extend.extend_file(&self.words_directory).await,
            Commands::Sort(sort) => sort.sort_file(self).await,
            Commands::Graph(graph) => graph.generate_graph(self, None).await,
        }?;
        self.log_and_print(
            &format!("Command {} executed successfully !", self.command),
            LogLevel::Info,
        )?;
        let end = Local::now();
        let diff = end - start;
        self.log_and_print(
            &format!(
                "This operation took {}s {}ms",
                diff.num_seconds(),
                diff.num_milliseconds() % 100
            ),
            LogLevel::Info,
        )?;
        Ok(())
    }

    pub fn verify(&self) -> Result<()> {
        if let Some(log) = self.log.as_ref() {
            if !PathBuf::from(log).try_exists()? {
                OpenOptions::new().create(true).write(true).open(log)?;
            }
        }
        Ok(())
    }

    pub fn log(&self, msg: &str, level: LogLevel) -> Result<()> {
        if let Some(log) = self.log.as_ref() {
            let mut file = OpenOptions::new().create(true).append(true).open(log)?;
            let date = Local::now();
            file.write("]".as_bytes())?;
            file.write(format!("{}", date.format(LOG_FORMAT)).as_bytes())?;
            file.write("] ".as_bytes())?;
            file.write(level.to_string().as_bytes())?;
            file.write(" : ".as_bytes())?;
            file.write_all(msg.as_bytes())?;
            file.write(&[10])?;
        }
        Ok(())
    }

    pub fn log_and_print(&self, msg: &str, level: LogLevel) -> Result<()> {
        match level {
            LogLevel::Info => println!("{}", msg),
            _ => eprintln!("{}", msg),
        }
        self.log(msg, level)
    }
}
