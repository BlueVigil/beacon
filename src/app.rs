use std::path::PathBuf;

use gpui::{
   AsyncApp,
   Context,
   PathPromptOptions,
   Task,
   WeakEntity,
};

use crate::tycmd::{
   self,
   CommandOutput,
   TeensyDevice,
   Tycmd,
};

pub struct BeaconApp {
   pub selected_hex:          Option<PathBuf>,
   pub devices:               Vec<TeensyDevice>,
   pub selected_device_index: Option<usize>,
   pub identify_output:       Option<String>,
   pub output_lines:          Vec<String>,
   pub status:                AppStatus,
   pub active_task:           Option<Task<()>>,
   pub tycmd:                 Option<Tycmd>,
   pub expected_tycmd_path:   PathBuf,
   pub blink_visible:         bool,
   blink_task:                Option<Task<()>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppStatus {
   Idle,
   SelectingFile,
   Detecting,
   Identifying,
   Ready,
   Uploading,
   Success,
   Error(AppErrorKind),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppErrorKind {
   MissingTycmd(PathBuf),
   InvalidHexFile(PathBuf),
   NoDevice,
   MultipleDevicesNoSelection,
   CommandFailed {
      command:   String,
      exit_code: Option<i32>,
   },
   Io(String),
}

impl BeaconApp {
   pub fn new(_cx: &mut Context<Self>) -> Self {
      let expected_tycmd_path = tycmd::expected_resource_path();
      let mut app = Self {
         selected_hex:          None,
         devices:               Vec::new(),
         selected_device_index: None,
         identify_output:       None,
         output_lines:          Vec::new(),
         status:                AppStatus::Idle,
         active_task:           None,
         tycmd:                 None,
         expected_tycmd_path:   expected_tycmd_path.clone(),
         blink_visible:         true,
         blink_task:            None,
      };

      match Tycmd::resolve() {
         Ok(tycmd) => {
            app.log(format!("tycmd sidecar: {}", tycmd.executable().display()));
            app.tycmd = Some(tycmd);
         },
         Err(error) => {
            app.log_error(format!("missing tycmd sidecar: {error}"));
            app.status = AppStatus::Error(AppErrorKind::MissingTycmd(expected_tycmd_path));
         },
      }

      app
   }

   pub fn choose_hex(
      &mut self,
      _event: &gpui::ClickEvent,
      _window: &mut gpui::Window,
      cx: &mut Context<Self>,
   ) {
      if self.is_busy() {
         return;
      }

      self.status = AppStatus::SelectingFile;
      self.log("choose firmware .hex");
      cx.notify();

      let receiver = cx.prompt_for_paths(PathPromptOptions {
         files:       true,
         directories: false,
         multiple:    false,
         prompt:      Some("Choose firmware .hex".into()),
      });

      let task = cx.spawn(
         async move |this: WeakEntity<BeaconApp>, cx: &mut AsyncApp| {
            let selection = receiver.await;

            let _ = this.update(cx, |this, cx| {
               this.active_task = None;

               match selection {
                  Ok(Ok(Some(paths))) => {
                     if this.accept_selected_paths(paths) {
                        this.start_identify(cx);
                     }
                  },
                  Ok(Ok(None)) => {
                     this.log("file selection cancelled");
                     this.refresh_ready_status();
                  },
                  Ok(Err(error)) => {
                     this.status = AppStatus::Error(AppErrorKind::Io(error.to_string()));
                     this.log_error(format!("file picker failed: {error}"));
                  },
                  Err(error) => {
                     this.status = AppStatus::Error(AppErrorKind::Io(error.to_string()));
                     this.log_error(format!("file picker channel closed: {error}"));
                  },
               }

               cx.notify();
            });
         },
      );

      self.active_task = Some(task);
   }

   pub fn scan_devices(
      &mut self,
      _event: &gpui::ClickEvent,
      _window: &mut gpui::Window,
      cx: &mut Context<Self>,
   ) {
      self.start_scan(cx);
   }

   pub fn select_device(
      &mut self,
      index: usize,
      _window: &mut gpui::Window,
      cx: &mut Context<Self>,
   ) {
      if self.is_busy() || index >= self.devices.len() {
         return;
      }

      self.selected_device_index = Some(index);
      self.log(format!("selected device: {}", self.devices[index].label));
      self.refresh_ready_status();
      cx.notify();
   }

   pub fn upload(
      &mut self,
      _event: &gpui::ClickEvent,
      _window: &mut gpui::Window,
      cx: &mut Context<Self>,
   ) {
      if self.is_busy() {
         return;
      }

      let Some(tycmd) = self.tycmd.clone() else {
         self.status =
            AppStatus::Error(AppErrorKind::MissingTycmd(self.expected_tycmd_path.clone()));
         self.log_error("cannot upload: bundled tycmd is missing");
         cx.notify();
         return;
      };

      let Some(hex_path) = self.selected_hex.clone() else {
         self.status = AppStatus::Error(AppErrorKind::InvalidHexFile(PathBuf::from(
            "NO HEX SELECTED",
         )));
         self.log_error("cannot upload: no .hex file selected");
         cx.notify();
         return;
      };

      if self.selected_device_index.is_none() {
         self.status = AppStatus::Error(AppErrorKind::MultipleDevicesNoSelection);
         self.log_error("cannot upload: select a Teensy first");
         cx.notify();
         return;
      }

      self.status = AppStatus::Uploading;
      self.blink_visible = true;
      self.log_command(format!("tycmd upload {}", hex_path.display()));
      cx.notify();

      let blink_task = cx.spawn(
         async move |this: WeakEntity<BeaconApp>, cx: &mut AsyncApp| {
            'outer: loop {
               for _ in 0..10u8 {
                  for visible in [true, false] {
                     let still_uploading = this
                        .update(cx, |this, cx| {
                           if matches!(this.status, AppStatus::Uploading) {
                              this.blink_visible = visible;
                              cx.notify();
                              true
                           } else {
                              false
                           }
                        })
                        .unwrap_or(false);

                     if !still_uploading {
                        break 'outer;
                     }

                     cx.background_executor()
                        .timer(std::time::Duration::from_millis(60))
                        .await;
                  }
               }

               cx.background_executor()
                  .timer(std::time::Duration::from_millis(800))
                  .await;
            }
         },
      );

      self.blink_task = Some(blink_task);

      let task = cx.spawn(
         async move |this: WeakEntity<BeaconApp>, cx: &mut AsyncApp| {
            let result: anyhow::Result<CommandOutput> = cx
               .background_executor()
               .spawn(async move { tycmd.upload(&hex_path) })
               .await;

            let _ = this.update(cx, |this, cx| {
               match result {
                  Ok(output) => this.finish_upload(output),
                  Err(error) => {
                     this.status = AppStatus::Error(AppErrorKind::Io(error.to_string()));
                     this.log_error(format!("upload failed before command completed: {error}"));
                  },
               }

               this.active_task = None;
               this.blink_task = None;
               this.blink_visible = true;
               cx.notify();
            });
         },
      );

      self.active_task = Some(task);
   }

   pub fn is_busy(&self) -> bool {
      matches!(
         self.status,
         AppStatus::SelectingFile
            | AppStatus::Detecting
            | AppStatus::Identifying
            | AppStatus::Uploading
      )
   }

   pub fn can_upload(&self) -> bool {
      self.tycmd.is_some()
         && self.selected_hex.is_some()
         && self.selected_device_index.is_some()
         && !self.is_busy()
   }

   fn start_scan(&mut self, cx: &mut Context<Self>) {
      if self.is_busy() {
         return;
      }

      let Some(tycmd) = self.tycmd.clone() else {
         self.status =
            AppStatus::Error(AppErrorKind::MissingTycmd(self.expected_tycmd_path.clone()));
         self.log_error("cannot scan: bundled tycmd is missing");
         cx.notify();
         return;
      };

      self.status = AppStatus::Detecting;
      self.devices.clear();
      self.selected_device_index = None;
      self.log_command("tycmd list");
      cx.notify();

      let task = cx.spawn(
         async move |this: WeakEntity<BeaconApp>, cx: &mut AsyncApp| {
            let result: anyhow::Result<CommandOutput> = cx
               .background_executor()
               .spawn(async move { tycmd.list() })
               .await;

            let _ = this.update(cx, |this, cx| {
               this.active_task = None;

               match result {
                  Ok(output) => this.finish_scan(output),
                  Err(error) => {
                     this.status = AppStatus::Error(AppErrorKind::Io(error.to_string()));
                     this.log_error(format!("scan failed before command completed: {error}"));
                  },
               }

               cx.notify();
            });
         },
      );

      self.active_task = Some(task);
   }

   fn finish_scan(&mut self, output: CommandOutput) {
      self.append_command_output(&output);

      if !output.is_success() {
         self.status = AppStatus::Error(AppErrorKind::CommandFailed {
            command:   "tycmd list".to_string(),
            exit_code: output.status_code,
         });
         self.log_error(command_failure_line("tycmd list", &output));
         return;
      }

      self.devices = tycmd::parse_devices(&output.stdout);

      match self.devices.len() {
         0 => {
            self.selected_device_index = None;
            self.status = AppStatus::Error(AppErrorKind::NoDevice);
            self.log_error("no Teensy devices detected");
         },
         1 => {
            self.selected_device_index = Some(0);
            self.log(format!("detected device: {}", self.devices[0].label));
            self.refresh_ready_status();
         },
         count => {
            self.selected_device_index = None;
            self.status = AppStatus::Error(AppErrorKind::MultipleDevicesNoSelection);
            self.log(format!(
               "{count} devices detected; select one before upload"
            ));
         },
      }
   }

   fn start_identify(&mut self, cx: &mut Context<Self>) {
      let Some(tycmd) = self.tycmd.clone() else {
         self.refresh_ready_status();
         return;
      };

      let Some(hex_path) = self.selected_hex.clone() else {
         self.refresh_ready_status();
         return;
      };

      self.status = AppStatus::Identifying;
      self.identify_output = Some("checking firmware file...".to_string());
      self.log_command(format!("tycmd identify {}", hex_path.display()));
      cx.notify();

      let task = cx.spawn(
         async move |this: WeakEntity<BeaconApp>, cx: &mut AsyncApp| {
            let result: anyhow::Result<CommandOutput> = cx
               .background_executor()
               .spawn(async move { tycmd.identify(&hex_path) })
               .await;

            let _ = this.update(cx, |this, cx| {
               match result {
                  Ok(output) => this.finish_identify(output),
                  Err(error) => {
                     this.identify_output = Some(format!("identify failed: {error}"));
                     this.log_error(format!("identify failed before command completed: {error}"));
                     this.refresh_ready_status();
                  },
               }

               this.active_task = None;
               cx.notify();
            });
         },
      );

      self.active_task = Some(task);
   }

   fn finish_identify(&mut self, output: CommandOutput) {
      self.append_command_output(&output);

      let combined = combined_output(&output);
      self.identify_output = Some(if combined.trim().is_empty() {
         "identify completed with no output".to_string()
      } else {
         combined
      });

      if output.is_success() {
         self.log("identify complete");
      } else {
         self.log_error(command_failure_line("tycmd identify", &output));
      }

      self.refresh_ready_status();
   }

   fn finish_upload(&mut self, output: CommandOutput) {
      self.append_command_output(&output);

      if output.is_success() {
         self.status = AppStatus::Success;
         self.log_success("upload succeeded");
      } else {
         self.status = AppStatus::Error(AppErrorKind::CommandFailed {
            command:   "tycmd upload".to_string(),
            exit_code: output.status_code,
         });
         self.log_error(command_failure_line("tycmd upload", &output));
      }
   }

   fn accept_selected_paths(&mut self, paths: Vec<PathBuf>) -> bool {
      let Some(path) = paths.into_iter().next() else {
         self.log("file selection cancelled");
         self.refresh_ready_status();
         return false;
      };

      if !tycmd::is_hex_file(&path) {
         self.status = AppStatus::Error(AppErrorKind::InvalidHexFile(path.clone()));
         self.log_error(format!("rejected non-hex file: {}", path.display()));
         return false;
      }

      self.selected_hex = Some(path.clone());
      self.identify_output = None;
      self.log_success(format!("selected firmware: {}", path.display()));
      true
   }

   fn refresh_ready_status(&mut self) {
      if self.tycmd.is_none() {
         self.status =
            AppStatus::Error(AppErrorKind::MissingTycmd(self.expected_tycmd_path.clone()));
      } else if self.selected_hex.is_some() && self.selected_device_index.is_some() {
         self.status = AppStatus::Ready;
      } else {
         self.status = AppStatus::Idle;
      }
   }

   fn append_command_output(&mut self, output: &CommandOutput) {
      append_prefixed_lines(&mut self.output_lines, "OUT", &output.stdout);
      append_prefixed_lines(&mut self.output_lines, "ERR", &output.stderr);
   }

   fn log(&mut self, line: impl Into<String>) {
      self.output_lines.push(format!("INFO {}", line.into()));
      self.truncate_log();
   }

   fn log_command(&mut self, command: impl Into<String>) {
      self.output_lines.push(format!("$ {}", command.into()));
      self.truncate_log();
   }

   fn log_success(&mut self, line: impl Into<String>) {
      self.output_lines.push(format!("OK {}", line.into()));
      self.truncate_log();
   }

   fn log_error(&mut self, line: impl Into<String>) {
      self.output_lines.push(format!("ERR {}", line.into()));
      self.truncate_log();
   }

   fn truncate_log(&mut self) {
      const MAX_LINES: usize = 200;

      if self.output_lines.len() > MAX_LINES {
         self
            .output_lines
            .drain(0..self.output_lines.len() - MAX_LINES);
      }
   }
}

fn append_prefixed_lines(lines: &mut Vec<String>, prefix: &str, output: &str) {
   for line in output.lines() {
      lines.push(format!("{prefix} {line}"));
   }
}

fn combined_output(output: &CommandOutput) -> String {
   match (output.stdout.trim(), output.stderr.trim()) {
      ("", "") => String::new(),
      (stdout, "") => stdout.to_string(),
      ("", stderr) => stderr.to_string(),
      (stdout, stderr) => format!("{stdout}\n{stderr}"),
   }
}

fn command_failure_line(command: &str, output: &CommandOutput) -> String {
   format!("{command} failed with exit {:?}", output.status_code)
}
