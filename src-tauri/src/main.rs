#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use std::process::Command;
use tauri::Manager;  // to manage tauri events

// Command to execute in Alacritty
#[tauri::command]
fn execute_command(cmd: String) -> String {
  // Launch Alacritty with the given command
  let output = Command::new("alacritty")
      .arg("-e") // to run the command in Alacritty's shell
      .arg(cmd)
      .output();

  // Process the result
  match output {
      Ok(output) => {
          let stdout = String::from_utf8_lossy(&output.stdout);
          let stderr = String::from_utf8_lossy(&output.stderr);

          if !stderr.is_empty() {
              return format!("Error: {}", stderr);
          }
          return stdout.to_string();
      },
      Err(e) => format!("Failed to execute command: {}", e),
  }
}

fn main() {
  tauri::Builder::default()
      .invoke_handler(tauri::generate_handler![execute_command])
      .run(tauri::generate_context!())
      .expect("error while running tauri application");
}

