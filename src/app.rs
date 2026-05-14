// SPDX-License-Identifier: AGPL-3.0-only

use std::{
   fs,
   path::{
      Path,
      PathBuf,
   },
   sync::mpsc,
};

use gpui::{
   AsyncApp,
   Context,
   FocusHandle,
   Focusable,
   PathPromptOptions,
   Task,
   WeakEntity,
};

use crate::{
   actions,
   tnc_rs::{
      self,
      Device,
      UploadOptions,
   },
};

pub struct BeaconApp {
   pub selected_hex:          Option<PathBuf>,
   pub devices:               Vec<Device>,
   pub selected_device_index: Option<usize>,
   pub identify_output:       Option<String>,
   pub output_lines:          Vec<String>,
   pub status:                AppStatus,
   pub active_task:           Option<Task<()>>,
   pub blink_visible:         bool,
   pub chevron_tick:          u32,
   pub auto_mode:             AutoMode,
   pub(crate) focus_handle:   FocusHandle,
   chevron_anim_phase:        u8,
   blink_task:                Option<Task<()>>,
   auto_scan_task:            Option<Task<()>>,
   _chevron_anim_task:        Task<()>,
   upload_triggered_by_auto:  bool,
   last_auto_upload_device:   Option<String>,
   open_urls_task:            Option<Task<()>>,
   last_hex_dir:              Option<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppStatus {
   Idle,
   SelectingFile,
   Detecting,
   Identifying,
   AutoWaiting,
   Ready,
   Uploading,
   Success,
   Error(AppErrorKind),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppErrorKind {
   InvalidHexFile(PathBuf),
   NoDevice,
   MultipleDevicesNoSelection,
   Io(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutoMode {
   Off,
   Wait,
   Instant,
}

impl BeaconApp {
   pub fn new(cx: &mut Context<Self>, open_urls: mpsc::Receiver<Vec<String>>) -> Self {
      let last_hex_dir = read_last_hex_dir();

      let chevron_anim_task = cx.spawn(
         async move |this: WeakEntity<BeaconApp>, cx: &mut AsyncApp| {
            loop {
               cx.background_executor()
                  .timer(std::time::Duration::from_millis(110))
                  .await;

               let ok = this
                  .update(cx, |this, cx| {
                     if this.current_chevron_phase() != 0 {
                        this.chevron_tick = this.chevron_tick.wrapping_add(1);
                        cx.notify();
                     }
                  })
                  .is_ok();

               if !ok {
                  break;
               }
            }
         },
      );

      let mut app = Self {
         selected_hex: None,
         devices: Vec::new(),
         selected_device_index: None,
         identify_output: None,
         output_lines: Vec::new(),
         status: AppStatus::Idle,
         active_task: None,
         blink_visible: true,
         chevron_tick: 0,
         auto_mode: AutoMode::Off,
         focus_handle: cx.focus_handle(),
         chevron_anim_phase: 0,
         blink_task: None,
         auto_scan_task: None,
         _chevron_anim_task: chevron_anim_task,
         upload_triggered_by_auto: false,
         last_auto_upload_device: None,
         open_urls_task: None,
         last_hex_dir,
      };

      app.start_open_urls_task(cx, open_urls);
      app.log("tnc-rs native backend active");
      app.start_auto_scan(cx);

      app
   }

   pub fn load_hex_action(
      &mut self,
      _action: &actions::LoadHex,
      _window: &mut gpui::Window,
      cx: &mut Context<Self>,
   ) {
      self.load_hex(cx);
   }

   fn load_hex(&mut self, cx: &mut Context<Self>) {
      if self.is_busy() {
         return;
      }

      self.status = AppStatus::SelectingFile;
      if let Some(dir) = &self.last_hex_dir {
         self.log(format!(
            "choose firmware .hex (last directory: {})",
            dir.display()
         ));
      } else {
         self.log("choose firmware .hex");
      }
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

               this.sync_chevron_phase();
               cx.notify();
            });
         },
      );

      self.active_task = Some(task);
   }

   pub fn drop_hex_paths(
      &mut self,
      paths: &gpui::ExternalPaths,
      _window: &mut gpui::Window,
      cx: &mut Context<Self>,
   ) {
      if self.is_busy() {
         return;
      }

      if self.accept_selected_paths(paths.paths().to_vec()) {
         self.start_identify(cx);
      }

      self.sync_chevron_phase();
      cx.notify();
   }

   pub fn scan_usb_action(
      &mut self,
      _action: &actions::ScanUsb,
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
      self.last_auto_upload_device = None;
      self.log(format!("selected device: {}", self.devices[index].label));
      self.refresh_ready_status();
      self.sync_chevron_phase();
      cx.notify();
   }

   pub fn upload_action(
      &mut self,
      _action: &actions::Upload,
      _window: &mut gpui::Window,
      cx: &mut Context<Self>,
   ) {
      self.upload_selected(cx);
   }

   fn upload_selected(&mut self, cx: &mut Context<Self>) {
      if !self.is_busy() {
         self.do_upload(cx);
      }
   }

   pub fn cycle_auto_mode_action(
      &mut self,
      _action: &actions::CycleAutoMode,
      _window: &mut gpui::Window,
      cx: &mut Context<Self>,
   ) {
      self.cycle_auto(cx);
   }

   pub fn quit_action(
      &mut self,
      _action: &actions::Quit,
      _window: &mut gpui::Window,
      cx: &mut Context<Self>,
   ) {
      cx.quit();
   }

   fn cycle_auto(&mut self, cx: &mut Context<Self>) {
      if self.is_busy() {
         return;
      }

      self.auto_mode = match self.auto_mode {
         AutoMode::Off => AutoMode::Wait,
         AutoMode::Wait => AutoMode::Instant,
         AutoMode::Instant => AutoMode::Off,
      };

      match self.auto_mode {
         AutoMode::Wait => {
            if self.selected_hex.is_some() && !self.devices.is_empty() && !self.is_busy() {
               self.log("auto-wait: device present, uploading...");
               self.start_auto_wait(cx);
            } else if self.selected_hex.is_some() {
               self.log("auto-wait: armed, waiting for device...");
            } else {
               self.log("auto-wait: load a .hex to arm");
               self.auto_mode = AutoMode::Off;
            }
         },
         AutoMode::Instant => {
            if self.can_upload() {
               self.log("auto-instant: device present, uploading now...");
               self.upload_triggered_by_auto = true;
               self.remember_auto_upload_device();
               self.do_upload(cx);
            } else if self.selected_hex.is_some() {
               self.log("auto-instant: armed, waiting for device...");
            } else {
               self.log("auto-instant: load a .hex to arm");
               self.auto_mode = AutoMode::Off;
            }
         },
         AutoMode::Off => {
            self.log("auto: off");
            self.upload_triggered_by_auto = false;
            self.blink_task = None;
            self.blink_visible = true;
            if matches!(self.status, AppStatus::Uploading | AppStatus::AutoWaiting) {
               self.refresh_ready_status();
            }
         },
      }

      cx.notify();
   }

   fn do_upload(&mut self, cx: &mut Context<Self>) {
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
      self.log_command(format!("tnc-rs upload {}", hex_path.display()));
      cx.notify();

      let blink_task = cx.spawn(
         async move |this: WeakEntity<BeaconApp>, cx: &mut AsyncApp| {
            'outer: loop {
               for _ in 0..10u8 {
                  for visible in [true, false] {
                     let still_uploading = this
                        .update(cx, |this, cx| {
                           if matches!(this.status, AppStatus::AutoWaiting) {
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
            let (sender, receiver) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
               let _ = sender.send(tnc_rs::upload_firmware(
                  &hex_path,
                  &UploadOptions::default(),
               ));
            });

            let result: anyhow::Result<()> = loop {
               match receiver.try_recv() {
                  Ok(result) => break result,
                  Err(std::sync::mpsc::TryRecvError::Empty) => {
                     cx.background_executor()
                        .timer(std::time::Duration::from_millis(100))
                        .await;
                  },
                  Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                     break Err(anyhow::anyhow!("upload thread disconnected"));
                  },
               }
            };

            let _ = this.update(cx, |this, cx| {
               match result {
                  Ok(()) => {
                     let was_auto = this.upload_triggered_by_auto;
                     this.finish_upload();
                     if was_auto {
                        this.upload_triggered_by_auto = false;
                        if this.auto_mode == AutoMode::Instant {
                           this.log("auto-instant: waiting for next device...");
                        }
                     }
                  },
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

   fn start_auto_wait(&mut self, cx: &mut Context<Self>) {
      let Some(hex_path) = self.selected_hex.clone() else {
         return;
      };

      self.upload_triggered_by_auto = true;
      self.status = AppStatus::AutoWaiting;
      self.blink_visible = true;
      self.log_command(format!("tnc-rs upload --wait {}", hex_path.display()));
      cx.notify();

      let blink_task = cx.spawn(
         async move |this: WeakEntity<BeaconApp>, cx: &mut AsyncApp| {
            'outer: loop {
               for _ in 0..10u8 {
                  for visible in [true, false] {
                     let still_uploading = this
                        .update(cx, |this, cx| {
                           if matches!(this.status, AppStatus::AutoWaiting) {
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
            let (sender, receiver) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
               let options = UploadOptions {
                  wait: true,
                  ..UploadOptions::default()
               };
               let _ = sender.send(tnc_rs::upload_firmware(&hex_path, &options));
            });

            let result: anyhow::Result<()> = loop {
               match receiver.try_recv() {
                  Ok(result) => break result,
                  Err(std::sync::mpsc::TryRecvError::Empty) => {
                     cx.background_executor()
                        .timer(std::time::Duration::from_millis(100))
                        .await;
                  },
                  Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                     break Err(anyhow::anyhow!(
                        "auto-wait worker exited before reporting result"
                     ));
                  },
               }
            };

            let _ = this.update(cx, |this, cx| {
               match result {
                  Ok(()) => {
                     this.status = AppStatus::Success;
                     this.log_success("auto-wait: upload succeeded");
                  },
                  Err(error) => {
                     this.status = AppStatus::Error(AppErrorKind::Io(error.to_string()));
                     this.log_error(format!("auto-wait: {error}"));
                  },
               }

               if this.auto_mode == AutoMode::Wait {
                  this.log("auto-wait: waiting for next device...");
               }

               this.active_task = None;
               this.blink_task = None;
               this.blink_visible = true;
               this.upload_triggered_by_auto = false;
               this.sync_chevron_phase();
               cx.notify();
            });
         },
      );

      self.active_task = Some(task);
   }

   fn start_auto_scan(&mut self, cx: &mut Context<Self>) {
      let task = cx.spawn(
         async move |this: WeakEntity<BeaconApp>, cx: &mut AsyncApp| {
            loop {
               cx.background_executor()
                  .timer(std::time::Duration::from_secs(1))
                  .await;

               let result = tnc_rs::list_devices();

               if this
                  .update(cx, |this, cx| {
                     let Ok(devices) = result else {
                        this.log_error(format!("scan failed: {}", result.unwrap_err()));
                        return;
                     };
                     let old_lines: Vec<String> =
                        this.devices.iter().map(|d| d.raw_line.clone()).collect();
                     let new_lines: Vec<String> =
                        devices.iter().map(|d| d.raw_line.clone()).collect();

                     if old_lines == new_lines {
                        return;
                     }

                     let old_count = this.devices.len();
                     this.devices = devices;

                     if this.devices.len() > old_count {
                        this.log(format!(
                           "auto-scan: {} device(s) detected",
                           this.devices.len()
                        ));
                     }

                     if this.devices.len() == 1 && this.selected_device_index.is_none() {
                        this.selected_device_index = Some(0);
                     }

                     if let Some(idx) = this.selected_device_index
                        && idx >= this.devices.len()
                     {
                        this.selected_device_index = None;
                     }

                     this.refresh_ready_status();

                     if this.auto_mode == AutoMode::Instant
                        && !this.devices.is_empty()
                        && this.selected_hex.is_some()
                        && !this.is_busy()
                        && this.active_task.is_none()
                        && this.auto_upload_device_changed()
                     {
                        this.log("auto-instant: device detected, uploading...");
                        this.upload_triggered_by_auto = true;
                        this.remember_auto_upload_device();
                        this.do_upload(cx);
                     }

                     if this.auto_mode == AutoMode::Wait
                        && !this.devices.is_empty()
                        && this.selected_hex.is_some()
                        && !this.is_busy()
                        && this.active_task.is_none()
                     {
                        this.log("auto-wait: device detected, starting upload...");
                        this.start_auto_wait(cx);
                     }

                     this.sync_chevron_phase();
                     cx.notify();
                  })
                  .is_err()
               {
                  break;
               }
            }
         },
      );

      self.auto_scan_task = Some(task);
   }

   fn start_open_urls_task(
      &mut self,
      cx: &mut Context<Self>,
      receiver: mpsc::Receiver<Vec<String>>,
   ) {
      let task = cx.spawn(
         async move |this: WeakEntity<BeaconApp>, cx: &mut AsyncApp| {
            loop {
               match receiver.try_recv() {
                  Ok(urls) => {
                     let paths: Vec<PathBuf> =
                        urls.into_iter().filter_map(path_from_open_url).collect();
                     let _ = this.update(cx, |this, cx| {
                        if this.is_busy() {
                           this.log("open file ignored: app is busy");
                           return;
                        }

                        if this.accept_selected_paths(paths) {
                           this.start_identify(cx);
                        }

                        this.sync_chevron_phase();
                        cx.notify();
                     });
                  },
                  Err(mpsc::TryRecvError::Empty) => {
                     cx.background_executor()
                        .timer(std::time::Duration::from_millis(200))
                        .await;
                  },
                  Err(mpsc::TryRecvError::Disconnected) => break,
               }
            }
         },
      );

      self.open_urls_task = Some(task);
   }

   pub fn current_chevron_phase(&self) -> u8 {
      if self.can_upload()
         || matches!(self.status, AppStatus::Uploading | AppStatus::AutoWaiting)
         || self.auto_mode != AutoMode::Off
      {
         2
      } else if self.selected_hex.is_some() && self.selected_device_index.is_none() {
         1
      } else {
         0
      }
   }

   pub fn sync_chevron_phase(&mut self) {
      let phase = self.current_chevron_phase();
      if phase != self.chevron_anim_phase {
         self.chevron_tick = 0;
         self.chevron_anim_phase = phase;
      }
   }

   pub fn can_upload(&self) -> bool {
      self.selected_hex.is_some()
         && self.selected_device_index.is_some()
         && !matches!(self.status, AppStatus::AutoWaiting)
         && !self.is_busy()
   }

   fn auto_upload_device_changed(&self) -> bool {
      self
         .current_auto_upload_device()
         .is_some_and(|device| self.last_auto_upload_device.as_deref() != Some(device.as_str()))
   }

   fn remember_auto_upload_device(&mut self) {
      self.last_auto_upload_device = self.current_auto_upload_device();
   }

   fn current_auto_upload_device(&self) -> Option<String> {
      self
         .selected_device_index
         .and_then(|index| self.devices.get(index))
         .or_else(|| {
            if self.devices.len() == 1 {
               self.devices.first()
            } else {
               None
            }
         })
         .map(|device| stable_device_identity(&device.raw_line))
   }

   fn start_scan(&mut self, cx: &mut Context<Self>) {
      if self.is_busy() {
         return;
      }

      self.status = AppStatus::Detecting;
      self.devices.clear();
      self.selected_device_index = None;
      self.log_command("tnc-rs list");
      cx.notify();

      let task = cx.spawn(
         async move |this: WeakEntity<BeaconApp>, cx: &mut AsyncApp| {
            let result = tnc_rs::list_devices();

            let _ = this.update(cx, |this, cx| {
               this.active_task = None;

               match result {
                  Ok(devices) => this.finish_scan(devices),
                  Err(error) => {
                     this.status = AppStatus::Error(AppErrorKind::Io(error.to_string()));
                     this.log_error(format!("scan failed: {error}"));
                  },
               }

               this.sync_chevron_phase();
               cx.notify();
            });
         },
      );

      self.active_task = Some(task);
   }

   fn finish_scan(&mut self, devices: Vec<Device>) {
      self.devices = devices;

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
      let Some(hex_path) = self.selected_hex.clone() else {
         self.refresh_ready_status();
         return;
      };

      self.status = AppStatus::Identifying;
      self.identify_output = Some("checking firmware file...".to_string());
      self.log_command(format!("tnc-rs identify {}", hex_path.display()));
      cx.notify();

      let task = cx.spawn(
         async move |this: WeakEntity<BeaconApp>, cx: &mut AsyncApp| {
            let result = cx
               .background_executor()
               .spawn(async move { tnc_rs::identify_firmware_file(&hex_path) })
               .await;

            let _ = this.update(cx, |this, cx| {
               match result {
                  Ok(models) => this.finish_identify(models),
                  Err(error) => {
                     this.identify_output = Some(format!("identify failed: {error}"));
                     this.log_error(format!("identify failed before command completed: {error}"));
                     this.refresh_ready_status();
                  },
               }

               this.active_task = None;
               this.sync_chevron_phase();
               cx.notify();
            });
         },
      );

      self.active_task = Some(task);
   }

   fn finish_identify(&mut self, models: Vec<tnc_rs::Model>) {
      let output = if models.is_empty() {
         "Unknown".to_string()
      } else {
         models
            .iter()
            .map(|model| model.info().name)
            .collect::<Vec<_>>()
            .join(", ")
      };
      self.identify_output = Some(output.clone());
      self.log(format!("identify complete: {output}"));
      self.refresh_ready_status();
   }

   fn finish_upload(&mut self) {
      self.status = AppStatus::Success;
      self.log_success("upload succeeded");
   }

   fn accept_selected_paths(&mut self, paths: Vec<PathBuf>) -> bool {
      let Some(path) = paths.into_iter().next() else {
         self.log("file selection cancelled");
         self.refresh_ready_status();
         return false;
      };

      if !tnc_rs::is_hex_file(&path) {
         self.status = AppStatus::Error(AppErrorKind::InvalidHexFile(path.clone()));
         self.log_error(format!("rejected non-hex file: {}", path.display()));
         return false;
      }

      self.selected_hex = Some(path.clone());
      self.last_hex_dir = path.parent().map(PathBuf::from);
      if let Some(dir) = &self.last_hex_dir
         && let Err(error) = write_last_hex_dir(dir)
      {
         self.log_error(format!("could not save last firmware directory: {error}"));
      }
      self.identify_output = None;
      self.log_success(format!("selected firmware: {}", path.display()));
      true
   }

   fn refresh_ready_status(&mut self) {
      if self.selected_hex.is_some() && self.selected_device_index.is_some() {
         self.status = AppStatus::Ready;
      } else {
         self.status = AppStatus::Idle;
      }
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

impl Focusable for BeaconApp {
   fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
      self.focus_handle.clone()
   }
}

fn stable_device_identity(raw_line: &str) -> String {
   raw_line
      .split_whitespace()
      .next()
      .unwrap_or(raw_line)
      .to_string()
}

fn path_from_open_url(url: String) -> Option<PathBuf> {
   if let Some(path) = url.strip_prefix("file://") {
      return Some(PathBuf::from(percent_decode_path(path)));
   }

   Some(PathBuf::from(url)).filter(|path| path.exists())
}

fn percent_decode_path(path: &str) -> String {
   let bytes = path.as_bytes();
   let mut decoded = Vec::with_capacity(bytes.len());
   let mut index = 0;

   while index < bytes.len() {
      if bytes[index] == b'%'
         && index + 2 < bytes.len()
         && let Ok(hex) = std::str::from_utf8(&bytes[index + 1..index + 3])
         && let Ok(value) = u8::from_str_radix(hex, 16)
      {
         decoded.push(value);
         index += 3;
      } else {
         decoded.push(bytes[index]);
         index += 1;
      }
   }

   String::from_utf8_lossy(&decoded).into_owned()
}

fn settings_dir() -> Option<PathBuf> {
   let home = std::env::var_os("HOME").map(PathBuf::from)?;

   #[cfg(target_os = "macos")]
   return Some(
      home
         .join("Library")
         .join("Application Support")
         .join("BEACON"),
   );

   #[cfg(target_os = "windows")]
   return std::env::var_os("APPDATA")
      .map(PathBuf::from)
      .map(|path| path.join("BEACON"))
      .or_else(|| Some(home.join("AppData").join("Roaming").join("BEACON")));

   #[cfg(not(any(target_os = "macos", target_os = "windows")))]
   return std::env::var_os("XDG_CONFIG_HOME")
      .map(PathBuf::from)
      .map(|path| path.join("beacon"))
      .or_else(|| Some(home.join(".config").join("beacon")));
}

fn last_hex_dir_file() -> Option<PathBuf> {
   settings_dir().map(|dir| dir.join("last_hex_dir"))
}

fn read_last_hex_dir() -> Option<PathBuf> {
   let path = last_hex_dir_file()?;
   let dir = fs::read_to_string(path).ok()?;
   let dir = PathBuf::from(dir.trim());
   dir.is_dir().then_some(dir)
}

fn write_last_hex_dir(dir: &Path) -> std::io::Result<()> {
   let Some(path) = last_hex_dir_file() else {
      return Ok(());
   };
   if let Some(parent) = path.parent() {
      fs::create_dir_all(parent)?;
   }
   fs::write(path, dir.to_string_lossy().as_bytes())
}
