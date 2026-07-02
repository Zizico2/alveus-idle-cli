//! Stdio JSON-RPC carrier for the Bevy Remote Protocol.

use std::io::{self, BufRead, Write};

use async_channel::Sender;
use bevy::prelude::*;
use bevy::remote::{
    BrpBatch, BrpError, BrpMessage, BrpRequest, BrpResponse, BrpResult, BrpSender, error_codes,
};
use serde_json::Value;

pub struct StdioBrpPlugin;

impl Plugin for StdioBrpPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, start_stdio_server);
    }
}

fn start_stdio_server(request_sender: Res<BrpSender>) {
    let sender = request_sender.clone();
    std::thread::spawn(move || stdio_blocking_main(sender));
}

fn stdio_blocking_main(request_sender: Sender<BrpMessage>) {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let reader = stdin.lock();

    for line in reader.lines() {
        let Ok(line) = line else {
            break;
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<Value>(line) {
            Ok(value) => process_line(value, &request_sender),
            Err(err) => BrpResponse::new(
                None,
                Err(BrpError {
                    code: error_codes::PARSE_ERROR,
                    message: err.to_string(),
                    data: None,
                }),
            ),
        };

        if let Ok(serialized) = serde_json::to_string(&response) {
            let _ = writeln!(stdout, "{serialized}");
            let _ = stdout.flush();
        }
    }
}

fn process_line(value: Value, request_sender: &Sender<BrpMessage>) -> BrpResponse {
    match serde_json::from_value::<BrpBatch>(value.clone()) {
        Ok(BrpBatch::Single(request)) => process_single_request(request, request_sender),
        Ok(BrpBatch::Batch(requests)) => {
            let responses: Vec<Value> = requests
                .into_iter()
                .map(|request| {
                    serde_json::to_value(process_single_request(request, request_sender))
                        .unwrap_or(Value::Null)
                })
                .collect();
            BrpResponse::new(None, Ok(Value::Array(responses)))
        }
        Err(err) => BrpResponse::new(
            value.as_object().and_then(|map| map.get("id")).cloned(),
            Err(BrpError {
                code: error_codes::INVALID_REQUEST,
                message: err.to_string(),
                data: None,
            }),
        ),
    }
}

fn process_single_request(request: Value, request_sender: &Sender<BrpMessage>) -> BrpResponse {
    let id = request.as_object().and_then(|map| map.get("id")).cloned();

    let jsonrpc = request
        .as_object()
        .and_then(|map| map.get("jsonrpc"))
        .and_then(|value| value.as_str());
    if jsonrpc != Some("2.0") {
        return BrpResponse::new(
            id,
            Err(BrpError {
                code: error_codes::INVALID_REQUEST,
                message: String::from("JSON-RPC request requires `\"jsonrpc\": \"2.0\"`"),
                data: None,
            }),
        );
    }

    let request: BrpRequest = match serde_json::from_value(request) {
        Ok(v) => v,
        Err(err) => {
            return BrpResponse::new(
                id,
                Err(BrpError {
                    code: error_codes::INVALID_REQUEST,
                    message: err.to_string(),
                    data: None,
                }),
            );
        }
    };

    if request.method.contains("+watch") {
        return BrpResponse::new(
            request.id,
            Err(BrpError {
                code: error_codes::INVALID_REQUEST,
                message: String::from("Watching methods are not supported on stdio transport"),
                data: None,
            }),
        );
    }

    let (result_sender, result_receiver) = async_channel::bounded(1);

    if request_sender
        .send_blocking(BrpMessage {
            method: request.method,
            params: request.params,
            sender: result_sender,
        })
        .is_err()
    {
        return BrpResponse::new(
            request.id,
            Err(BrpError {
                code: error_codes::INTERNAL_ERROR,
                message: String::from("BRP request channel closed"),
                data: None,
            }),
        );
    }

    match result_receiver.recv_blocking() {
        Ok(result) => BrpResponse::new(request.id, result),
        Err(_) => BrpResponse::new(
            request.id,
            Err(BrpError {
                code: error_codes::INTERNAL_ERROR,
                message: String::from("BRP response channel closed"),
                data: None,
            }),
        ),
    }
}
