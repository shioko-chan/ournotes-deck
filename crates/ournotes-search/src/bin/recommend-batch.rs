//! Streaming native transport for the same owned-snapshot facade used by DeckSolver.
use ournotes_search::engine::recommend_snapshot;
use ournotes_sim::data::DeckData;
use serde::Deserialize;
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
use std::sync::{Arc, Mutex, mpsc};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Job {
    id: Value,
    snapshot: Value,
    request: Value,
}

fn answer(data: &DeckData, line: &str) -> Result<Value, serde_json::Error> {
    let job: Job = serde_json::from_str(line)?;
    Ok(json!({"id": job.id, "answer": recommend_snapshot(
        data, &job.snapshot.to_string(), &job.request.to_string(), None
    )}))
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mut deck = None;
    let mut threads = std::thread::available_parallelism()?.get();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--deck" => deck = Some(args.next().ok_or("--deck needs a path")?),
            "--threads" => threads = args.next().ok_or("--threads needs a count")?.parse()?,
            _ => return Err(format!("unknown argument {arg}; usage: recommend-batch --deck PATH [--threads N]").into()),
        }
    }
    if threads == 0 {
        return Err("--threads must be positive".into());
    }
    let data = DeckData::from_json(&std::fs::read_to_string(deck.ok_or("--deck is required")?)?)?;
    let (send, receive) = mpsc::sync_channel::<String>(threads);
    let receive = Arc::new(Mutex::new(receive));
    std::thread::scope(|scope| -> Result<(), Box<dyn std::error::Error>> {
        let mut workers = Vec::new();
        for _ in 0..threads {
            let receive = Arc::clone(&receive);
            let data = &data;
            workers.push(scope.spawn(move || -> Result<(), String> {
                loop {
                    let line = receive.lock().map_err(|e| e.to_string())?.recv();
                    let Ok(line) = line else { return Ok(()) };
                    // Malformed envelopes are transport errors; valid jobs retain the engine's error answer.
                    let result = answer(data, &line).map_err(|e| e.to_string())?;
                    let mut out = io::stdout().lock();
                    writeln!(out, "{result}").map_err(|e| e.to_string())?;
                    out.flush().map_err(|e| e.to_string())?;
                }
            }));
        }
        drop(receive);
        let input_result = (|| -> Result<(), Box<dyn std::error::Error>> {
            for line in io::stdin().lock().lines() {
                send.send(line?)?;
            }
            Ok(())
        })();
        drop(send);
        for worker in workers {
            worker.join().map_err(|_| "worker panicked")??;
        }
        input_result
    })
}

fn main() {
    if let Err(error) = run() {
        eprintln!("recommend-batch: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn answer_preserves_id_and_the_snapshot_facade_error() {
        let data = DeckData { provenance: Value::Null, sha256: None, master: Default::default(), charts: Vec::new() };
        let result = answer(&data, r#"{"id":[1,"job"],"snapshot":{},"request":{}}"#).unwrap();
        let expected = serde_json::to_value(recommend_snapshot(&data, "{}", "{}", None)).unwrap();
        assert_eq!(result, json!({"id": [1, "job"], "answer": expected}));
        assert_eq!(result["answer"]["status"], "invalid");
    }

    #[test]
    fn envelopes_require_all_fields_and_reject_unknown_fields() {
        assert!(serde_json::from_str::<Job>(r#"{"id":1,"snapshot":{}}"#).is_err());
        assert!(serde_json::from_str::<Job>(r#"{"id":1,"snapshot":{},"request":{},"typo":0}"#).is_err());
        let job: Job = serde_json::from_str(r#"{"id":{"key":1},"snapshot":{},"request":{}}"#).unwrap();
        assert_eq!(job.id, json!({"key":1}));
    }
}
