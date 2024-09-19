// src/main.rs
//

use std::{
    fs::{File, OpenOptions},
    io::{prelude::*, BufReader},
};

use regex::Regex;
use surrealdb::{
    engine::remote::ws::{Client, Ws},
    opt::{auth::Root, QueryStream},
    sql::Thing,
    Surreal,
};

use anyhow::Result;

// const DEFAULT_FILE_NAME: &str = "rc.surql";

// const DEFAULT_ADDRESS: &str = "localhost:8654";

// const DEFAULT_ROOT: &Root = &Root {
//     username: "root",
//     password: "root",
// };

// const DEFAULT_NS: &str = "job-seek";
// const DEFAULT_DB: &str = "development";

use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long, default_value = "localhost:8654")]
    address: String,

    #[arg(short, long, default_value = "root")]
    username: String,

    #[arg(short, long, default_value = "root")]
    password: String,

    #[arg(short, long, default_value = "job-seek")]
    namespace: String,

    #[arg(short, long, default_value = "development")]
    database: String,

    #[arg(short, long, default_value = "rc.surql")]
    file: String,
}

// const DB_CLI: &Surreal<Client> = nil;

#[tokio::main]
async fn main() -> Result<()> {
    // read arg
    // example : main.rs <filename> --address <address> --port <port>
    let args = Args::parse();

    // open file to file buffer
    let file = File::open(args.file)?;
    let buf_reader = BufReader::new(file);

    let mut commad_line = String::new();
    let mut in_command = false;
    let mut command_count = 0;
    let mut line_count = 0;
    let start_pattern = Regex::new(r"^\s+(UPDATE|UPSERT|CREATE|INSERT)").unwrap();
    let end_pattern = Regex::new(r";$").unwrap();

    let mut error_set: Vec<(i32, i32, String, String)> = Vec::new();

    let db_cli = Surreal::new::<Ws>(args.address).await?;
    db_cli
        .signin(Root {
            username: &args.username,
            password: &args.password,
        })
        .await?;
    db_cli.use_ns(args.namespace).use_db(args.database).await?;

    for line_result in buf_reader.lines() {
        if line_result.is_err() {
            break;
        }
        line_count += 1;
        let line = line_result.unwrap();
        if start_pattern.is_match(&line) {
            commad_line = line.clone();
            commad_line.push_str("\n");
            in_command = true;
        } else if end_pattern.is_match(&line) && in_command {
            commad_line.push_str(&line);
            commad_line.push_str("\n");
            match run_query(&db_cli, &commad_line).await {
                Ok(_) => {}
                Err(e) => error_set.push((
                    line_count,
                    command_count + 1,
                    commad_line.to_owned(),
                    e.to_string(),
                )),
            }

            commad_line.clear();
            in_command = false;
            command_count += 1;
            println!("command_count: {}", command_count);
            println!("in line: {}", line_count);

            if command_count % 100 == 0 {
                write_error_log(error_set.clone())?;
                error_set.clear();
            }
        } else if in_command {
            commad_line.push_str(&line);
            commad_line.push_str("\n");
        }
    }

    println!("error_set: {:?}", error_set);

    Ok(())
}

async fn run_query(db_cli: &Surreal<Client>, query_str: &str) -> surrealdb::Result<()> {
    let result = db_cli.query(query_str).await;
    match result {
        Ok(_) => {
            // println!("result: {:?}", result);
        }
        Err(e) => {
            println!("error: {:?}", e);
            return Err(e);
        }
    }

    Ok(())
}

fn write_error_log(error_set: Vec<(i32, i32, String, String)>) -> Result<()> {
    let mut log_file = OpenOptions::new()
        .write(true)
        .create(true)
        .open("error.log")?;
    let mut quury_file = OpenOptions::new()
        .write(true)
        .create(true)
        .open("fail_run.surql")?;

    // let mut file = File::open("error.log")?;
    for (line, command, query, error_str) in error_set {
        log_file.write_all(
            format!(
                "line: {}, command: {}, error: {} \n",
                line, command, error_str
            )
            .as_bytes(),
        )?;

        quury_file.write_all(format!("--- command: {}, \n {} \n", command, query).as_bytes())?;
    }

    Ok(())
}
