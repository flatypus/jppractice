use ansi_term::Style;
use chrono::prelude::*;
use colored::Colorize;
use rand::Rng;
use serde_json::Value;
use std::cmp::Ordering;
use std::io::BufRead;
use std::sync::{Arc, Mutex};
use std::{fs, io, path::Path, time::Duration};
use tokio::time;

#[derive(Clone)]
struct TimeStat<'a> {
    user_answer: String,
    time: f64,
    word: &'a str,
}

async fn input(prompt: Option<&str>) -> String {
    match prompt {
        Some(vprompt) => print!("{:?} ", vprompt),
        None => (),
    };

    let mut res = String::new();
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    handle.read_line(&mut res).unwrap();
    res.trim().to_string()
}

fn read_json_from_path(path: &str) -> Value {
    let path = Path::new(path);
    let json_string = fs::read_to_string(path.to_str().unwrap()).unwrap();
    serde_json::from_str(&json_string).unwrap()
}

fn print_time(time1: i64, time2: i64, chars: usize) {
    if chars == 0 {
        println!("{}", "Error: chars cannot be zero".red());
        return;
    }

    let elapsed_time_s = (time2 - time1) as f64 / 1000.00;
    let avg_time_per_char_s = elapsed_time_s / (chars as f64);

    println!("{:.2}s", elapsed_time_s);
    println!("Average time per character: {:.2}s", avg_time_per_char_s);
}

fn print_stats(times: Vec<TimeStat>) {
    if times.len() == 0 {
        println!("{}", "No values found!".red());
        return;
    }

    // Find the longest word length
    let longest_word_length = times
        .iter()
        .map(|timestat| timestat.word.chars().count() as i64 + timestat.user_answer.len() as i64)
        .max()
        .unwrap_or(0);

    let mut sorted_times = times.clone();
    sorted_times.sort_by(|a, b| {
        let a_rate = a.time / a.user_answer.len() as f64;
        let b_rate = b.time / b.user_answer.len() as f64;
        b_rate.partial_cmp(&a_rate).unwrap_or(Ordering::Equal)
    });

    for TimeStat {
        user_answer,
        time,
        word,
    } in sorted_times.iter()
    {
        println!(
            "{} {} {:.2}s {:.2}s/char",
            format!(
                "{:width$}",
                user_answer,
                width = longest_word_length as usize
            ),
            format!("{:width$}", word, width = longest_word_length as usize),
            time,
            time / user_answer.len() as f64
        );
    }

    const BARS: i32 = 15;

    for bar in (0..BARS).rev() {
        for (
            index,
            TimeStat {
                user_answer,
                time,
                word: _,
            },
        ) in times.iter().enumerate()
        {
            let char_count = user_answer.len() as f64;
            if time / char_count * BARS as f64 >= bar as f64 {
                print!("{}", "██".green());
            } else {
                print!("  ");
            }
            print!("{}", if index == times.len() - 1 { "\n" } else { " " });
        }
    }

    println!(
        "\nAverage time per character overall: {:.2}",
        times.iter().map(|timestat| timestat.time).sum::<f64>()
            / times
                .iter()
                .map(|timestat| timestat.user_answer.len() as f64)
                .sum::<f64>()
    );
}

async fn game(timeout_seconds: u64) {
    let words = read_json_from_path("resources/words.json");
    let words_length = words.as_array().unwrap().len();
    let mut rng = rand::thread_rng();
    const RIGHT: char = '✅';
    const WRONG: char = '❌';
    let mut times: Vec<TimeStat> = Vec::new();

    let timeout_flag = Arc::new(Mutex::new(false));
    let flag_clone = Arc::clone(&timeout_flag);

    tokio::spawn(async move {
        time::sleep(Duration::from_secs(timeout_seconds)).await;
        let mut flag = flag_clone.lock().unwrap();
        *flag = true;
    });

    loop {
        {
            let flag = timeout_flag.lock().unwrap();
            if *flag {
                println!("{}", "\nTime is up!".red());
                print_stats(times);
                break;
            }
        }

        let random_num = rng.gen_range(0..words_length);
        let random_obj = words[random_num].as_object().unwrap();
        let word = random_obj["word"].as_str().unwrap();
        let romaji = random_obj["romaji"].as_str().unwrap();
        let meaning = random_obj["meaning"].as_str().unwrap();
        println!("{} ({})", word, meaning);

        let time1 = Utc::now().timestamp_millis();
        let user_answer = input(None).await;
        let time2 = Utc::now().timestamp_millis();
        let lowercase_answer = romaji.to_lowercase();
        let check_against = lowercase_answer.split("(").collect::<Vec<&str>>()[0];

        if user_answer.to_lowercase() == check_against {
            println!("{} {}", lowercase_answer, RIGHT);
            print_time(time1, time2, user_answer.len());
            times.push({
                TimeStat {
                    user_answer: user_answer,
                    time: ((time2 - time1) as f64) / 1000.00,
                    word: word,
                }
            })
        } else {
            for (index, item) in user_answer.chars().enumerate() {
                if index >= lowercase_answer.len() {
                    break;
                }
                let nth = lowercase_answer.chars().nth(index).unwrap();
                if item == nth {
                    print!("{}", item);
                } else {
                    print!("{}", Style::new().underline().paint(item.to_string()));
                }
            }
            println!("({}) {}", lowercase_answer, WRONG);
        }
        println!();
    }
}

#[tokio::main]
async fn main() {
    const TIMEOUT_SECONDS: u64 = 60;
    game(TIMEOUT_SECONDS).await;
}
