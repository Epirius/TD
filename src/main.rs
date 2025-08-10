use chrono::{DateTime, Utc};
use clap::{Args, Parser, Subcommand};
use git2::Repository;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::{env, fs, io::{self, Write}, path::PathBuf};
use anyhow::{Result, anyhow};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum TaskStatus {
    TODO,
    DOING,
    DONE
}

#[derive(Debug, Serialize, Deserialize)]
struct TaskMetadata {
    pub title: String,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    pub id: Uuid,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

#[derive(Debug)]
struct Task {
    pub metadata: TaskMetadata,
    pub description: String
}

impl  Task {
    pub fn from_str(content: &str) -> Result<Self> {
        if !content.starts_with("---\n") {
            return Err(anyhow!("The task file does not start with '---' followed by a new line"))
        }
        let end_of_frontmatter = content[4..]
            .find("---\n")
            .ok_or_else(|| anyhow::anyhow!("Missing closing '---'"))?;
        let yaml_str = &content[4..4 + end_of_frontmatter];
        let description = content[4 + end_of_frontmatter + 4..].to_string();
        let metadata: TaskMetadata = serde_yaml::from_str(yaml_str)?;

        Ok(Task {
            metadata,
            description,
        })
    }

    pub fn to_string(&self) -> Result<String> {
        let yaml_str = serde_yaml::to_string(&self.metadata)?;

        // Combine the parts into the final file format.
        Ok(format!(
            "---\n{}---\n{}",
            yaml_str, self.description
        ))
    }

    pub fn new(args: &AddArgs) -> Self {
        Task { 
            metadata: TaskMetadata { 
                title: args.title.clone(), 
                status: TaskStatus::TODO, 
                created_at: Utc::now(), 
                id: Uuid::new_v4(), 
                tags: args.tags.as_ref()
                    .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                    .or(Some(Vec::new()))
                    .expect("Tags will be an empty list if none was given"),
                updated_at: None, 
            } ,
            description: args.desc.clone().or_else(|| Some(String::new())).expect("The description will be an empty string if none is given") }
    }
}



#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Adds a new task to the current project
    Add(AddArgs),
    /// List tasks
    Ls 
}

#[derive(Args, Debug)]
struct AddArgs {
    /// The title of the task
    title: String,
    /// A description of the task
    #[arg(long, short)]
    desc: Option<String>,
    // Comma-seperated list of tags
    #[arg(long, short)]
    tags: Option<String>
}

fn main() {
    let td_dir = create_td_home().unwrap();

    let cli = Cli::parse();

    dbg!(&cli.command);
    match &cli.command {
        Some(Commands::Add(args)) => {
            println!("add command");
            add_task(args)
        }
        Some(Commands::Ls) => {
            println!("ls command");
            list_task().unwrap()
        }
        /*Some(Commands::Ls { project }) => {
            if project.is_some() && project.clone().unwrap().is_empty() {
                fs::read_dir(td_dir_path).unwrap().for_each(|folder_content| {
                     let file = folder_content.unwrap().file_name().into_string().unwrap();
                     println!("{}", file)
            });
            }
        }*/
        None => {
            println!("No command provided. Use --help for more information.");
        }
    }
}

fn add_task(args: &AddArgs) {
    dbg!(args);
    println!("add");
    let mut project_dir = get_project_path().unwrap();
    project_dir.push("test_file.td");
    let mut task_file = std::fs::File::create(project_dir).unwrap();
    let _ = task_file.write(Task::new(args).to_string().unwrap().as_bytes());
}

fn list_task() -> Result<()> {
    println!("ls");
    let project_dir = get_project_path().unwrap();
    println!("project dir: {}", project_dir.to_str().unwrap());
    for entry in fs::read_dir(project_dir)? {
        let entry = entry?;
        let name = entry.file_name();
        println!("{}", name.to_str().ok_or(anyhow!("could not read file name"))?)
    }
    Ok(())
}

fn get_project_path() -> Result<PathBuf> {
    let mut project_dir = PathBuf::new();
    project_dir.push(dirs::home_dir().ok_or(anyhow!("Could not find the home directory"))?);
    project_dir.push(".td");
    if let Some(origin) = get_repo_remote() {
        project_dir.push(origin);
    }
    std::fs::create_dir_all(&project_dir)?;
    return Ok(project_dir)
}

fn create_td_home() -> io::Result<PathBuf> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Could not find home directory"))?;

    let td_dir_path = home_dir.join(".td");
    if !td_dir_path.exists() {
        fs::create_dir(&td_dir_path);
    }
    return Ok(td_dir_path);
}

fn get_repo_remote() -> Option<String> {
    match Repository::open_from_env() {
        Ok(repo) => {
            match repo.find_remote("origin") {
                Ok(remote) => match remote.url() {
                    Some(url) => Some(sanitize_dir_name(url)),
                    None => None
                }
                Err(err) => None
            }
        }
        Err(err) => None,
    }
}

fn sanitize_dir_name(origin: &str) -> String {
    let problematic_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|', ' ', '@', '#', '$', '%', '^', '&', '+', '=', '~'];
    let mut sanitized = origin.to_string();

    for &c in problematic_chars.iter() {
        sanitized = sanitized.replace(c, "_")
    }
    
    sanitized = sanitized.trim_matches('.').to_string();
    sanitized = sanitized.replace("..", "_");
    sanitized
}