use serde::Deserialize;
use serde_json::{json, Value};
use sonic_bridge::config::SonicConfig;
use sonic_bridge::pipeline::SonicPipeline;
use std::io::{self, BufRead, Write};
use std::path::Path;

#[derive(Deserialize)]
struct RpcRequest {
    id: Option<Value>,
    method: Option<String>,
    params: Option<Value>,
}

fn send_response(id: Value, result: Value) {
    let response = json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    });
    let mut stdout = io::stdout();
    writeln!(stdout, "{}", serde_json::to_string(&response).unwrap()).unwrap();
    stdout.flush().unwrap();
}

fn send_error(id: Option<Value>, code: i32, message: &str) {
    let id_val = id.unwrap_or(Value::Null);
    let response = json!({
        "jsonrpc": "2.0",
        "id": id_val,
        "error": {
            "code": code,
            "message": message
        }
    });
    let mut stdout = io::stdout();
    writeln!(stdout, "{}", serde_json::to_string(&response).unwrap()).unwrap();
    stdout.flush().unwrap();
}

fn main() {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }

        let req: Result<RpcRequest, _> = serde_json::from_str(&line);
        if let Ok(request) = req {
            let method = request.method.as_deref().unwrap_or("");
            match method {
                "initialize" => {
                    if let Some(id) = request.id {
                        send_response(
                            id,
                            json!({
                                "protocolVersion": "2024-11-05",
                                "capabilities": {
                                    "tools": {}
                                },
                                "serverInfo": {
                                    "name": "sonic-bridge-mcp",
                                    "version": "0.2.1"
                                }
                            }),
                        );
                    }
                }
                "notifications/initialized" => {
                    // Do nothing
                }
                "ping" => {
                    if let Some(id) = request.id {
                        send_response(id, json!({}));
                    }
                }
                "tools/list" => {
                    if let Some(id) = request.id {
                        send_response(
                            id,
                            json!({
                                "tools": [
                                    {
                                        "name": "analyze_music",
                                        "description": "Analyze an audio file (wav/mp3/flac) and extract musical features (Chord, Timbre, Dynamic). Returns LLM-readable text.",
                                        "inputSchema": {
                                            "type": "object",
                                            "properties": {
                                                "filepath": { "type": "string", "description": "Absolute path to the audio file" },
                                                "onset_mode": { "type": "boolean", "description": "Set to true to use dynamic onset-driven segmentation (good for complex music), false for fixed intervals." }
                                            },
                                            "required": ["filepath"]
                                        }
                                    },
                                    {
                                        "name": "compare_music",
                                        "description": "Compare two audio files using Dynamic Time Warping (DTW) to find sequence differences.",
                                        "inputSchema": {
                                            "type": "object",
                                            "properties": {
                                                "file_a": { "type": "string", "description": "Absolute path to the original audio file" },
                                                "file_b": { "type": "string", "description": "Absolute path to the cover/second audio file" }
                                            },
                                            "required": ["file_a", "file_b"]
                                        }
                                    }
                                ]
                            }),
                        );
                    }
                }
                "tools/call" => {
                    if let Some(id) = request.id {
                        let params = request.params.unwrap_or(json!({}));
                        let name = params["name"].as_str().unwrap_or("");
                        let args = params["arguments"].as_object();

                        if name == "analyze_music" {
                            if let Some(args) = args {
                                let filepath =
                                    args.get("filepath").and_then(|v| v.as_str()).unwrap_or("");
                                let onset_mode = args
                                    .get("onset_mode")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false);

                                let config = SonicConfig {
                                    onset_mode,
                                    ..Default::default()
                                };

                                match SonicPipeline::process_single(Path::new(filepath), &config) {
                                    Ok((meta, segs)) => {
                                        let mut report = Vec::new();
                                        report.push(
                                            "# SonicBridge: LLM-Readable Music Descriptor (LRMD)"
                                                .to_string(),
                                        );
                                        report.push(
                                            "## 1. Global Acoustic & Musicological Metadata"
                                                .to_string(),
                                        );
                                        report.push(format!("- **Filename**: `{}`", meta.filename));
                                        report.push(format!(
                                            "- **Duration**: `{:.2} seconds`",
                                            meta.duration_seconds
                                        ));
                                        report.push(format!(
                                            "- **Tempo (BPM)**: `{:.1} BPM` ({})",
                                            meta.estimated_bpm, meta.tempo_feeling
                                        ));
                                        report.push(format!(
                                            "- **Estimated Key**: `{}`\n",
                                            meta.estimated_global_key
                                        ));

                                        report.push(
                                            "## 2. Spatiotemporal Track Analysis".to_string(),
                                        );
                                        report.push("| Timeline | Chord | Dynamic Intensity | Timbral Brightness | Rhythmic & Transient Activity |".to_string());
                                        report.push(
                                            "| :--- | :--- | :--- | :--- | :--- |".to_string(),
                                        );

                                        for seg in &segs {
                                            report.push(format!(
                                                "| **{}** | `{}` | {} | {} | {} |",
                                                seg.time_range,
                                                seg.chord,
                                                seg.dynamic_level,
                                                seg.timbre_brightness,
                                                seg.rhythm_activity
                                            ));
                                        }

                                        send_response(
                                            id,
                                            json!({
                                                "content": [
                                                    {
                                                        "type": "text",
                                                        "text": report.join("\n")
                                                    }
                                                ]
                                            }),
                                        );
                                    }
                                    Err(e) => {
                                        send_response(
                                            id,
                                            json!({
                                                "isError": true,
                                                "content": [
                                                    {
                                                        "type": "text",
                                                        "text": format!("Error processing audio: {}", e)
                                                    }
                                                ]
                                            }),
                                        );
                                    }
                                }
                            } else {
                                send_error(Some(id), -32602, "Invalid params");
                            }
                        } else if name == "compare_music" {
                            if let Some(args) = args {
                                let file_a =
                                    args.get("file_a").and_then(|v| v.as_str()).unwrap_or("");
                                let file_b =
                                    args.get("file_b").and_then(|v| v.as_str()).unwrap_or("");

                                match SonicPipeline::process_comparative(
                                    Path::new(file_a),
                                    Path::new(file_b),
                                ) {
                                    Ok(report_text) => {
                                        send_response(
                                            id,
                                            json!({
                                                "content": [
                                                    {
                                                        "type": "text",
                                                        "text": report_text
                                                    }
                                                ]
                                            }),
                                        );
                                    }
                                    Err(e) => {
                                        send_response(
                                            id,
                                            json!({
                                                "isError": true,
                                                "content": [
                                                    {
                                                        "type": "text",
                                                        "text": format!("Error processing audio: {}", e)
                                                    }
                                                ]
                                            }),
                                        );
                                    }
                                }
                            } else {
                                send_error(Some(id), -32602, "Invalid params");
                            }
                        } else {
                            send_error(Some(id), -32601, "Tool not found");
                        }
                    }
                }
                _ => {
                    if let Some(id) = request.id {
                        send_error(Some(id), -32601, "Method not found");
                    }
                }
            }
        }
    }
}
