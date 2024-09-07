#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use std::process::{Command, Stdio};
use std::thread;
use tokio::runtime::Runtime;
use warp::Filter;
use warp::http::Method;
use serde_json::Value;

use std::env;
use std::sync::{Arc, Mutex};
use tauri::State;

#[derive(Clone)]
struct AppState {
    current_dir: Arc<Mutex<String>>,
}

#[tauri::command]
fn run_command(state: State<AppState>, input: String) -> Result<String, String> {
    let mut current_dir = state.current_dir.lock().unwrap(); // Khóa và truy cập current_dir

    if input.starts_with("cd") {
        // Handle `cd` command
        let path = input.split_whitespace().nth(1).unwrap_or("");
        if path.is_empty() {
            *current_dir = env::current_dir().unwrap().display().to_string();
        } else if let Err(_) = env::set_current_dir(path) {
            return Err(format!("Failed to change directory to {}", path));
        } else {
            *current_dir = env::current_dir().unwrap().display().to_string();
        }
        return Ok(current_dir.clone());
    }

    // Thực thi lệnh
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(&["/C", &input])
            .current_dir(&*current_dir) // Thực thi trong thư mục hiện tại
            .output()
            .expect("failed to execute command")
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(&input)
            .current_dir(&*current_dir) // Thực thi trong thư mục hiện tại
            .output()
            .expect("failed to execute command")
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !stderr.is_empty() {
        Err(stderr.to_string())
    } else {
        Ok(stdout.to_string())
    }
}

fn main() {
    // Khai báo state bên ngoài để sử dụng cho cả Warp và Tauri
    let state = AppState {
        current_dir: Arc::new(Mutex::new(env::current_dir().unwrap().display().to_string())),
    };

    // Chạy server warp trong thread riêng
    let state_clone = state.clone(); // Clone state để chia sẻ giữa các thread
    thread::spawn(move || {
        // Tạo runtime cho ngữ cảnh async
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let cors = warp::cors()
                .allow_any_origin()  // Cho phép tất cả các nguồn gốc
                .allow_methods(&[Method::GET, Method::POST])  // Cho phép các phương thức HTTP
                .allow_headers(vec!["Content-Type"]);  // Các header được cho phép

            let run_command = warp::path("run_command")
                .and(warp::post())
                .and(warp::body::json())
                .and(warp::any().map(move || state_clone.clone())) // Truyền state vào warp
                .map(|input: Value, state: AppState| {
                    let command = input["input"].as_str().unwrap_or("");

                    let mut current_dir = state.current_dir.lock().unwrap(); // Khóa và truy cập current_dir

                    // Thực thi lệnh
                    let output = if cfg!(target_os = "windows") {
                        Command::new("cmd")
                            .args(&["/C", &command])
                            .current_dir(&*current_dir) // Thực thi trong thư mục hiện tại
                            .output()
                            .expect("failed to execute command")
                    } else {
                        Command::new("sh")
                            .arg("-c")
                            .arg(command)
                            .current_dir(&*current_dir) // Thực thi trong thư mục hiện tại
                            .output()
                            .expect("failed to execute command")
                    };

                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                    // Trả về phản hồi JSON
                    if !stderr.is_empty() {
                        warp::reply::json(&serde_json::json!({ "error": stderr }))
                    } else {
                        warp::reply::json(&serde_json::json!({ "output": stdout }))
                    }
                })
                .with(cors);  // Đính kèm CORS filter

            warp::serve(run_command)
                .run(([127, 0, 0, 1], 3030))
                .await;
        });
    });

    // Khởi chạy ứng dụng Tauri
    tauri::Builder::default()
        .manage(state) // Quản lý state trong Tauri
        .invoke_handler(tauri::generate_handler![run_command])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
