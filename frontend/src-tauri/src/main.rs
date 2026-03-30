// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Operation {
    Convert,
    Remux,
    Compress,
    Resize,
    Trim,
    ExtractAudio,
    Gif,
    Rotate,
    Watermark,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConvertParams {
    output_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CompressParams {
    crf: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResizeParams {
    width: i32,
    height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TrimParams {
    start_time: f64,
    duration: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GifParams {
    fps: i32,
    scale: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RotateParams {
    angle: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WatermarkParams {
    text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum OperationParams {
    Convert(ConvertParams),
    Compress(CompressParams),
    Resize(ResizeParams),
    Trim(TrimParams),
    Gif(GifParams),
    Rotate(RotateParams),
    Watermark(WatermarkParams),
    None(HashMap<String, serde_json::Value>),
}

impl Default for OperationParams {
    fn default() -> Self {
        OperationParams::None(HashMap::new())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProcessRequest {
    operation: Operation,
    #[serde(default)]
    params: OperationParams,
    file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProcessResponse {
    success: bool,
    job_id: Option<String>,
    result_path: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JobStatus {
    status: String,
    progress: i32,
    error: Option<String>,
}

struct AppState {
    jobs: Arc<Mutex<HashMap<String, JobStatus>>>,
    temp_dir: PathBuf,
}

impl Default for AppState {
    fn default() -> Self {
        let temp_dir = std::env::temp_dir().join("ffmpeg_webui");
        fs::create_dir_all(&temp_dir).ok();
        
        Self {
            jobs: Arc::new(Mutex::new(HashMap::new())),
            temp_dir,
        }
    }
}

fn get_output_ext(operation: &Operation, params: &OperationParams) -> String {
    match operation {
        Operation::Convert | Operation::Remux => {
            if let OperationParams::Convert(p) = params {
                p.output_format.clone()
            } else {
                "mp4".to_string()
            }
        }
        Operation::ExtractAudio => "mp3".to_string(),
        Operation::Gif => "gif".to_string(),
        _ => "mp4".to_string(),
    }
}

fn build_ffmpeg_command(
    input_path: &str,
    output_path: &str,
    operation: &Operation,
    params: &OperationParams,
) -> Vec<String> {
    let mut cmd = vec![
        "ffmpeg".to_string(),
        "-y".to_string(),
        "-i".to_string(),
        input_path.to_string(),
    ];

    match operation {
        Operation::Convert => {
            if let OperationParams::Convert(p) = params {
                let output_ext = &p.output_format;
                match output_ext.as_str() {
                    "mp4" | "avi" | "mkv" | "mov" => {
                        cmd.extend([
                            "-c:v".to_string(),
                            "libx264".to_string(),
                            "-preset".to_string(),
                            "fast".to_string(),
                            "-c:a".to_string(),
                            "aac".to_string(),
                            output_path.to_string(),
                        ]);
                    }
                    "webm" => {
                        cmd.extend([
                            "-c:v".to_string(),
                            "libvpx-vp9".to_string(),
                            "-c:a".to_string(),
                            "libopus".to_string(),
                            output_path.to_string(),
                        ]);
                    }
                    _ => {
                        cmd.extend(["-c".to_string(), "copy".to_string(), output_path.to_string()]);
                    }
                }
            } else {
                cmd.extend(["-c".to_string(), "copy".to_string(), output_path.to_string()]);
            }
        }
        Operation::Remux => {
            cmd.extend(["-c".to_string(), "copy".to_string(), output_path.to_string()]);
        }
        Operation::Compress => {
            let crf = if let OperationParams::Compress(p) = params {
                p.crf
            } else {
                23
            };
            cmd.extend([
                "-vcodec".to_string(),
                "libx264".to_string(),
                "-crf".to_string(),
                crf.to_string(),
                "-c:a".to_string(),
                "copy".to_string(),
                output_path.to_string(),
            ]);
        }
        Operation::Resize => {
            let (width, height) = if let OperationParams::Resize(p) = params {
                (p.width, p.height)
            } else {
                (1280, 720)
            };
            cmd.extend([
                "-vf".to_string(),
                format!("scale={}:{}", width, height),
                "-c:a".to_string(),
                "copy".to_string(),
                output_path.to_string(),
            ]);
        }
        Operation::Trim => {
            let (start_time, duration) = if let OperationParams::Trim(p) = params {
                (p.start_time, p.duration)
            } else {
                (0.0, 10.0)
            };
            cmd.extend([
                "-ss".to_string(),
                start_time.to_string(),
                "-t".to_string(),
                duration.to_string(),
                "-c".to_string(),
                "copy".to_string(),
                output_path.to_string(),
            ]);
        }
        Operation::ExtractAudio => {
            cmd.extend([
                "-vn".to_string(),
                "-acodec".to_string(),
                "libmp3lame".to_string(),
                "-q:a".to_string(),
                "2".to_string(),
                output_path.to_string(),
            ]);
        }
        Operation::Gif => {
            let (fps, scale) = if let OperationParams::Gif(p) = params {
                (p.fps, p.scale)
            } else {
                (15, 320)
            };
            cmd = vec![
                "ffmpeg".to_string(),
                "-y".to_string(),
                "-i".to_string(),
                input_path.to_string(),
                "-vf".to_string(),
                format!(
                    "fps={},scale={}:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse",
                    fps, scale
                ),
                "-loop".to_string(),
                "0".to_string(),
                output_path.to_string(),
            ];
        }
        Operation::Rotate => {
            let angle = if let OperationParams::Rotate(p) = params {
                p.angle
            } else {
                90
            };
            cmd.extend([
                "-vf".to_string(),
                format!("transpose={}", angle / 90),
                "-c:a".to_string(),
                "copy".to_string(),
                output_path.to_string(),
            ]);
        }
        Operation::Watermark => {
            let text = if let OperationParams::Watermark(p) = params {
                p.text.clone()
            } else {
                "FFmpeg Studio".to_string()
            };
            cmd.extend([
                "-vf".to_string(),
                format!(
                    "drawtext=text='{}':fontcolor=white:fontsize=24:box=1:boxcolor=0x00000099:boxborderw=5:x=(w-text_w)/2:y=(h-text_h)/2",
                    text
                ),
                "-c:a".to_string(),
                "copy".to_string(),
                output_path.to_string(),
            ]);
        }
    }

    cmd
}

#[tauri::command]
async fn process_video(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ProcessRequest,
) -> Result<ProcessResponse, String> {
    println!("[INFO] Received process_video request for: {}", request.file_path);
    log::info!("Received process_video request for: {}", request.file_path);
    
    let job_id = Uuid::new_v4().to_string();
    let input_path = request.file_path;

    if !std::path::Path::new(&input_path).exists() {
        eprintln!("[ERROR] Input file not found: {}", input_path);
        log::error!("Input file not found: {}", input_path);
        return Ok(ProcessResponse {
            success: false,
            job_id: None,
            result_path: None,
            error: Some("Input file not found".to_string()),
        });
    }

    let output_ext = get_output_ext(&request.operation, &request.params);
    println!("[DEBUG] Output extension: {}", output_ext);
    log::debug!("Output extension: {}", output_ext);

    let input_name = std::path::Path::new(&input_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    
    let operation_name = match request.operation {
        Operation::Convert => "convert",
        Operation::Remux => "remux",
        Operation::Compress => "compress",
        Operation::Resize => "resize",
        Operation::Trim => "trim",
        Operation::ExtractAudio => "audio",
        Operation::Gif => "gif",
        Operation::Rotate => "rotate",
        Operation::Watermark => "watermark",
    };

    let output_filename = format!("{}_{}.{}", input_name, operation_name, output_ext);
    let output_path = state.temp_dir.join(&output_filename);
    let output_path_str = output_path.to_string_lossy().to_string();
    
    println!("[INFO] Output path: {}", output_path_str);
    log::info!("Output path: {}", output_path_str);

    {
        let mut jobs = state.jobs.lock().unwrap();
        jobs.insert(
            job_id.clone(),
            JobStatus {
                status: "processing".to_string(),
                progress: 0,
                error: None,
            },
        );
    }

    let cmd = build_ffmpeg_command(&input_path, &output_path_str, &request.operation, &request.params);

    let job_id_clone = job_id.clone();
    let app_clone = app.clone();
    let jobs_clone = state.jobs.clone();

    tokio::spawn(async move {
        println!("[INFO] Starting FFmpeg process: {:?}", cmd);
        log::info!("Starting FFmpeg process: {:?}", cmd);
        
        let mut child = match Command::new(&cmd[0])
            .args(&cmd[1..])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[ERROR] Failed to spawn FFmpeg: {}", e);
                log::error!("Failed to spawn FFmpeg: {}", e);
                let mut jobs = jobs_clone.lock().unwrap();
                if let Some(job) = jobs.get_mut(&job_id_clone) {
                    job.status = "failed".to_string();
                    job.error = Some(format!("Failed to spawn FFmpeg: {}", e));
                }
                let _ = app_clone.emit(
                    "progress",
                    JobStatus {
                        status: "failed".to_string(),
                        progress: 0,
                        error: Some(format!("Failed to spawn FFmpeg: {}", e)),
                    },
                );
                return;
            }
        };

        let stderr = match child.stderr.take() {
            Some(s) => s,
            None => {
                return;
            }
        };
        let mut reader = BufReader::new(stderr).lines();

        while let Ok(Some(line)) = reader.next_line().await {
            if line.contains("time=") {
                if let Some(caps) = regex::Regex::new(r"time=(\d+):(\d+):(\d+\.\d+)")
                    .ok()
                    .and_then(|re| re.captures(&line))
                {
                    let hours: f64 = caps.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0.0);
                    let minutes: f64 = caps.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0.0);
                    let seconds: f64 = caps.get(3).and_then(|m| m.as_str().parse().ok()).unwrap_or(0.0);
                    let total_seconds = hours * 3600.0 + minutes * 60.0 + seconds;
                    
                    let progress = std::cmp::min(95, (total_seconds * 10.0) as i32);
                    
                    let mut jobs = jobs_clone.lock().unwrap();
                    if let Some(job) = jobs.get_mut(&job_id_clone) {
                        job.progress = progress;
                    }

                    let _ = app_clone.emit(
                        "progress",
                        JobStatus {
                            status: "processing".to_string(),
                            progress,
                            error: None,
                        },
                    );
                }
            }
        }

        let status = child.wait().await;

        let mut jobs = jobs_clone.lock().unwrap();
        if let Some(job) = jobs.get_mut(&job_id_clone) {
            match status {
                Ok(s) if s.success() => {
                    job.status = "completed".to_string();
                    job.progress = 100;
                    let _ = app_clone.emit(
                        "progress",
                        JobStatus {
                            status: "completed".to_string(),
                            progress: 100,
                            error: None,
                        },
                    );
                }
                _ => {
                    job.status = "failed".to_string();
                    job.error = Some("FFmpeg process failed".to_string());
                    let _ = app_clone.emit(
                        "progress",
                        JobStatus {
                            status: "failed".to_string(),
                            progress: 0,
                            error: Some("FFmpeg process failed".to_string()),
                        },
                    );
                }
            }
        }
    });

    Ok(ProcessResponse {
        success: true,
        job_id: Some(job_id),
        result_path: Some(output_path_str),
        error: None,
    })
}

#[tauri::command]
fn get_status(state: State<'_, AppState>, job_id: String) -> Result<JobStatus, String> {
    let jobs = state.jobs.lock().unwrap();
    match jobs.get(&job_id) {
        Some(job) => Ok(job.clone()),
        None => Ok(JobStatus {
            status: "not_found".to_string(),
            progress: 0,
            error: Some("Job not found".to_string()),
        }),
    }
}

#[tauri::command]
async fn save_result(source_path: String, dest_path: String) -> Result<(), String> {
    println!("[INFO] Saving result from {} to {}", source_path, dest_path);
    log::info!("Saving result from {} to {}", source_path, dest_path);
    fs::copy(&source_path, &dest_path).map_err(|e| {
        eprintln!("[ERROR] Failed to save result: {}", e);
        log::error!("Failed to save result: {}", e);
        e.to_string()
    })?;
    println!("[INFO] Result saved successfully");
    log::info!("Result saved successfully");
    Ok(())
}

fn main() {
    eprintln!("[INFO] FFmpeg Studio starting...");
    
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("[PANIC] {}", panic_info);
    }));
    
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Debug)
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Stdout,
                ))
                .build(),
        )
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![process_video, get_status, save_result])
        .run(tauri::generate_context!());
    
    if let Err(e) = result {
        eprintln!("[ERROR] Failed to run Tauri application: {}", e);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_output_ext_convert() {
        let params = OperationParams::Convert(ConvertParams { output_format: "mp4".to_string() });
        assert_eq!(get_output_ext(&Operation::Convert, &params), "mp4");
    }

    #[test]
    fn test_get_output_ext_convert_webm() {
        let params = OperationParams::Convert(ConvertParams { output_format: "webm".to_string() });
        assert_eq!(get_output_ext(&Operation::Convert, &params), "webm");
    }

    #[test]
    fn test_get_output_ext_extract_audio() {
        let params = OperationParams::None(HashMap::new());
        assert_eq!(get_output_ext(&Operation::ExtractAudio, &params), "mp3");
    }

    #[test]
    fn test_get_output_ext_gif() {
        let params = OperationParams::None(HashMap::new());
        assert_eq!(get_output_ext(&Operation::Gif, &params), "gif");
    }

    #[test]
    fn test_get_output_ext_default() {
        let params = OperationParams::None(HashMap::new());
        assert_eq!(get_output_ext(&Operation::Compress, &params), "mp4");
    }

    #[test]
    fn test_build_ffmpeg_command_convert_mp4() {
        let params = OperationParams::Convert(ConvertParams { output_format: "mp4".to_string() });
        let cmd = build_ffmpeg_command("input.mp4", "output.mp4", &Operation::Convert, &params);
        
        assert!(cmd.contains(&"ffmpeg".to_string()));
        assert!(cmd.contains(&"-y".to_string()));
        assert!(cmd.contains(&"-i".to_string()));
        assert!(cmd.contains(&"input.mp4".to_string()));
        assert!(cmd.contains(&"output.mp4".to_string()));
        assert!(cmd.contains(&"-c:v".to_string()));
        assert!(cmd.contains(&"libx264".to_string()));
    }

    #[test]
    fn test_build_ffmpeg_command_convert_webm() {
        let params = OperationParams::Convert(ConvertParams { output_format: "webm".to_string() });
        let cmd = build_ffmpeg_command("input.mp4", "output.webm", &Operation::Convert, &params);
        
        assert!(cmd.contains(&"-c:v".to_string()));
        assert!(cmd.contains(&"libvpx-vp9".to_string()));
    }

    #[test]
    fn test_build_ffmpeg_command_compress() {
        let params = OperationParams::Compress(CompressParams { crf: 23 });
        let cmd = build_ffmpeg_command("input.mp4", "output.mp4", &Operation::Compress, &params);
        
        assert!(cmd.contains(&"-crf".to_string()));
        assert!(cmd.contains(&"23".to_string()));
    }

    #[test]
    fn test_build_ffmpeg_command_resize() {
        let params = OperationParams::Resize(ResizeParams { width: 1920, height: 1080 });
        let cmd = build_ffmpeg_command("input.mp4", "output.mp4", &Operation::Resize, &params);
        
        assert!(cmd.contains(&"-vf".to_string()));
        assert!(cmd.contains(&"scale=1920:1080".to_string()));
    }

    #[test]
    fn test_build_ffmpeg_command_trim() {
        let params = OperationParams::Trim(TrimParams { start_time: 5.0, duration: 10.0 });
        let cmd = build_ffmpeg_command("input.mp4", "output.mp4", &Operation::Trim, &params);
        
        assert!(cmd.contains(&"-ss".to_string()));
        assert!(cmd.contains(&"5".to_string()));
        assert!(cmd.contains(&"-t".to_string()));
        assert!(cmd.contains(&"10".to_string()));
    }

    #[test]
    fn test_build_ffmpeg_command_extract_audio() {
        let params = OperationParams::None(HashMap::new());
        let cmd = build_ffmpeg_command("input.mp4", "output.mp3", &Operation::ExtractAudio, &params);
        
        assert!(cmd.contains(&"-vn".to_string()));
        assert!(cmd.contains(&"-acodec".to_string()));
        assert!(cmd.contains(&"libmp3lame".to_string()));
    }

    #[test]
    fn test_build_ffmpeg_command_gif() {
        let params = OperationParams::Gif(GifParams { fps: 15, scale: 320 });
        let cmd = build_ffmpeg_command("input.mp4", "output.gif", &Operation::Gif, &params);
        
        let cmd_str = cmd.join(" ");
        assert!(cmd_str.contains("fps=15"));
        assert!(cmd_str.contains("scale=320"));
        assert!(cmd_str.contains("-loop"));
    }

    #[test]
    fn test_build_ffmpeg_command_rotate() {
        let params = OperationParams::Rotate(RotateParams { angle: 90 });
        let cmd = build_ffmpeg_command("input.mp4", "output.mp4", &Operation::Rotate, &params);
        
        assert!(cmd.contains(&"-vf".to_string()));
        assert!(cmd.contains(&"transpose=1".to_string()));
    }

    #[test]
    fn test_build_ffmpeg_command_watermark() {
        let params = OperationParams::Watermark(WatermarkParams { text: "Test".to_string() });
        let cmd = build_ffmpeg_command("input.mp4", "output.mp4", &Operation::Watermark, &params);
        
        let cmd_str = cmd.join(" ");
        assert!(cmd_str.contains("drawtext"));
        assert!(cmd_str.contains("text='Test'"));
    }

    #[test]
    fn test_app_state_default() {
        let state = AppState::default();
        assert!(state.jobs.lock().unwrap().is_empty());
        assert!(state.temp_dir.to_string_lossy().contains("ffmpeg_webui"));
    }

    #[test]
    fn test_operation_params_default() {
        let params = OperationParams::default();
        match params {
            OperationParams::None(_) => (),
            _ => panic!("Expected OperationParams::None"),
        }
    }
}
