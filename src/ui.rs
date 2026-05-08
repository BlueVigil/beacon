use gpui::{
   App,
   Context,
   FontWeight,
   InteractiveElement,
   IntoElement,
   ParentElement,
   Render,
   SharedString,
   StatefulInteractiveElement,
   Styled,
   Window,
   div,
   prelude::FluentBuilder,
   px,
   rgb,
};

use crate::{
   app::{
      AppErrorKind,
      AppStatus,
      BeaconApp,
   },
   theme::*,
};

impl Render for BeaconApp {
   fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
      div()
         .size_full()
         .bg(rgb(BG))
         .flex()
         .justify_center()
         .p_4()
         .child(
            div()
               .w(px(900.0))
               .h_full()
               .border_1()
               .border_color(rgb(BORDER_ACTIVE))
               .bg(rgb(BG))
               .p_4()
               .flex()
               .flex_col()
               .gap_3()
               .child(self.instrument_cluster())
               .child(
                  div()
                     .flex()
                     .gap_3()
                     .flex_1()
                     .child(
                        div()
                           .w(px(420.0))
                           .flex()
                           .flex_col()
                           .gap_3()
                           .child(self.firmware_panel(cx))
                           .child(self.device_panel(cx))
                           .child(self.verify_panel()),
                     )
                     .child(
                        div()
                           .flex_1()
                           .flex()
                           .flex_col()
                           .gap_3()
                           .child(self.upload_panel(cx))
                           .child(self.output_panel())
                           .child(self.recovery_panel()),
                     ),
               ),
         )
   }
}

const FONT_MONO: &str = "Courier New";

impl BeaconApp {
   fn instrument_cluster(&self) -> impl IntoElement {
      let error_state = matches!(self.status, AppStatus::Error(_));

      div()
         .h(px(150.0))
         .border_1()
         .border_color(rgb(if error_state { BORDER_ALERT } else { BORDER }))
         .bg(rgb(SURFACE))
         .flex()
         .flex_col()
         .child(
            div()
               .h(px(2.0))
               .bg(rgb(if error_state { ALERT } else { PHOSPHOR_DIM })),
         )
         .child(
            // Top status bar
            div()
               .flex()
               .items_center()
               .justify_between()
               .px_3()
               .py_2()
               .child(
                  div()
                     .flex()
                     .items_center()
                     .gap_2()
                     .child(anno("SN:001"))
                     .child(
                        div()
                           .font_family(FONT_MONO)
                           .text_size(px(10.0))
                           .text_color(if error_state {
                              rgb(ALERT)
                           } else {
                              rgb(PHOSPHOR)
                           })
                           .font_weight(FontWeight::SEMIBOLD)
                           .child(self.status_text()),
                     ),
               )
               .child(
                  div()
                     .flex()
                     .items_center()
                     .gap_3()
                     .child(led("SYS", true, PHOSPHOR))
                     .child(led("HEX", self.selected_hex.is_some(), AMBER))
                     .child(led("DEV", self.selected_device_index.is_some(), PHOSPHOR))
                     .child(led("FLT", error_state, ALERT))
                     .child(anno("24VDC")),
               ),
         )
         .child(
            // Main readout row
            div()
               .flex_1()
               .flex()
               .items_center()
               .justify_between()
               .px_4()
               .child(readout(
                  &format!("{:02}", self.devices.len().min(99)),
                  "DEVICES.DETECTED",
               ))
               .child(spectrum_analyzer())
               .child(readout(
                  if self.selected_hex.is_some() {
                     "ARMED"
                  } else {
                     "STBY"
                  },
                  "FIRMWARE.HEX",
               )),
         )
         .child(
            // Bottom info bar
            div()
               .flex()
               .items_center()
               .justify_between()
               .px_3()
               .py_2()
               .border_t_1()
               .border_color(rgb(BORDER))
               .child(
                  div()
                     .flex()
                     .flex_col()
                     .gap_1()
                     .child(anno("CLASS B"))
                     .child(
                        div()
                           .flex()
                           .gap_2()
                           .child(level_bar("LNK", self.tycmd.is_some(), PHOSPHOR))
                           .child(level_bar("HEX", self.selected_hex.is_some(), AMBER))
                           .child(level_bar("RDY", self.can_upload(), PHOSPHOR))
                           .child(level_bar(
                              "UP",
                              matches!(self.status, AppStatus::Uploading),
                              PHOSPHOR,
                           )),
                     ),
               )
               .child(
                  div()
                     .flex()
                     .flex_col()
                     .items_end()
                     .gap_2()
                     .child(
                        div()
                           .font_family(FONT_MONO)
                           .text_size(px(9.0))
                           .text_color(rgb(TEXT_DIM))
                           .child("TEENSY FIRMWARE LOADER  v0.1.0"),
                     )
                     .child(anno("MFG:USA")),
               ),
         )
   }

   fn firmware_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
      panel("MODULE.01  FIRMWARE", self.selected_hex.is_some(), AMBER)
         .child(data_field(
            "FILE",
            self
               .selected_hex
               .as_ref()
               .map(|p| p.display().to_string())
               .unwrap_or_else(|| "NO HEX SELECTED".into()),
            self.selected_hex.is_some(),
         ))
         .child(action_button(
            "LOAD HEX",
            self.is_busy(),
            cx.listener(Self::choose_hex),
         ))
   }

   fn device_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
      let selected = self
         .selected_device_index
         .and_then(|i| self.devices.get(i))
         .map(|d| d.label.as_str())
         .unwrap_or("NO TEENSY");

      let mut p = panel(
         "MODULE.02  DEVICE",
         self.selected_device_index.is_some(),
         PHOSPHOR,
      )
      .child(data_field(
         "STATUS",
         device_status(self),
         self.selected_device_index.is_some(),
      ))
      .child(data_field(
         "TARGET",
         selected,
         self.selected_device_index.is_some(),
      ))
      .child(action_button(
         "SCAN USB",
         self.is_busy(),
         cx.listener(Self::scan_devices),
      ));

      if self.devices.len() > 1 {
         let rows = self.devices.iter().enumerate().map(|(index, device)| {
            let selected = self.selected_device_index == Some(index);
            let id = SharedString::from(format!("device-row-{index}"));

            div()
               .id(id)
               .cursor_pointer()
               .border_1()
               .border_color(rgb(if selected { PHOSPHOR } else { BORDER }))
               .bg(rgb(if selected { PHOSPHOR_DARK } else { BG }))
               .px_3()
               .py_2()
               .flex()
               .items_center()
               .justify_between()
               .child(
                  div()
                     .font_family(FONT_MONO)
                     .text_size(px(10.0))
                     .text_color(rgb(if selected { PHOSPHOR } else { TEXT_DIM }))
                     .overflow_hidden()
                     .text_ellipsis()
                     .child(device.label.clone()),
               )
               .child(
                  div()
                     .w(px(6.0))
                     .h(px(6.0))
                     .bg(rgb(if selected { PHOSPHOR } else { BORDER })),
               )
               .when(!self.is_busy(), |row| {
                  row.on_click(cx.listener(move |this, _event, window, cx| {
                     this.select_device(index, window, cx);
                  }))
               })
         });
         p = p.child(div().flex().flex_col().gap_1().children(rows));
      }

      p
   }

   fn verify_panel(&self) -> impl IntoElement {
      let identify = self
         .identify_output
         .as_ref()
         .map(|o| summarize(o))
         .unwrap_or_else(|| "PENDING IDENTIFY".to_string());

      panel(
         "MODULE.03  VERIFY",
         self.identify_output.is_some(),
         PHOSPHOR,
      )
      .child(data_block(identify, self.identify_output.is_some()))
   }

   fn upload_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
      let can_upload = self.can_upload();
      panel("MODULE.04  UPLOAD", can_upload, AMBER)
         .child(data_field("STATUS", upload_status(self), can_upload))
         .child(action_button(
            "EXECUTE UPLOAD",
            !can_upload,
            cx.listener(Self::upload),
         ))
   }

   fn output_panel(&self) -> impl IntoElement {
      let lines = if self.output_lines.is_empty() {
         vec!["SYSTEM READY --- WAITING FOR COMMANDS".to_string()]
      } else {
         self.output_lines.iter().rev().take(12).cloned().collect()
      };

      panel("MODULE.05  OUTPUT", !self.output_lines.is_empty(), PHOSPHOR).child(
         div()
            .h(px(180.0))
            .border_1()
            .border_color(rgb(BORDER))
            .bg(rgb(BG))
            .p_3()
            .overflow_hidden()
            .flex()
            .flex_col()
            .gap_1()
            .children(lines.into_iter().rev().map(|line| {
               div()
                  .font_family(FONT_MONO)
                  .text_size(px(10.0))
                  .text_color(log_color(&line))
                  .overflow_hidden()
                  .text_ellipsis()
                  .child(line)
            })),
      )
   }

   fn recovery_panel(&self) -> impl IntoElement {
      let message = recovery_message(&self.status);
      let active = message.is_some();

      panel(
         "MODULE.06  RECOVERY",
         active || matches!(self.status, AppStatus::Success | AppStatus::Ready),
         if active { ALERT } else { PHOSPHOR },
      )
      .child(data_block(
         message.unwrap_or_else(|| {
            match self.status {
               AppStatus::Success => "UPLOAD COMPLETE".to_string(),
               AppStatus::Ready => "SYSTEM ARMED AND READY".to_string(),
               AppStatus::Uploading => "UPLOAD IN PROGRESS...".to_string(),
               _ => "STANDBY".to_string(),
            }
         }),
         active || matches!(self.status, AppStatus::Success | AppStatus::Ready),
      ))
   }
}

fn panel(title: &'static str, active: bool, accent: u32) -> gpui::Div {
   div()
      .border_1()
      .border_color(rgb(if active { accent } else { BORDER }))
      .bg(rgb(SURFACE))
      .p_3()
      .flex()
      .flex_col()
      .gap_2()
      .child(
         div()
            .flex()
            .items_center()
            .gap_2()
            .pb_2()
            .border_b_1()
            .border_color(rgb(if active { accent } else { BORDER }))
            .child(
               div()
                  .w(px(6.0))
                  .h(px(6.0))
                  .bg(rgb(if active { accent } else { BORDER })),
            )
            .child(
               div()
                  .font_family(FONT_MONO)
                  .text_size(px(10.0))
                  .text_color(rgb(if active { accent } else { TEXT }))
                  .font_weight(FontWeight::SEMIBOLD)
                  .child(title),
            ),
      )
}

fn anno(text: &str) -> impl IntoElement {
   div()
      .font_family(FONT_MONO)
      .text_size(px(8.0))
      .text_color(rgb(TEXT_DIM))
      .child(text.to_string())
}

fn action_button(
   label: &'static str,
   disabled: bool,
   listener: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
   div()
      .id(SharedString::from(format!("btn-{label}")))
      .cursor_pointer()
      .border_1()
      .border_color(rgb(if disabled { BORDER } else { PHOSPHOR }))
      .bg(rgb(if disabled { SURFACE } else { PHOSPHOR_DARK }))
      .px_3()
      .py_2()
      .flex()
      .items_center()
      .justify_center()
      .gap_2()
      .child(
         div()
            .w(px(6.0))
            .h(px(6.0))
            .bg(rgb(if disabled { BORDER } else { PHOSPHOR })),
      )
      .child(
         div()
            .text_size(px(11.0))
            .text_color(rgb(if disabled { TEXT_DIM } else { PHOSPHOR }))
            .font_family(FONT_MONO)
            .font_weight(FontWeight::SEMIBOLD)
            .child(label),
      )
      .when(!disabled, |btn| btn.on_click(listener))
}

fn data_field(label: &str, value: impl Into<String>, active: bool) -> impl IntoElement {
   div()
      .border_1()
      .border_color(rgb(if active { BORDER_ACTIVE } else { BORDER }))
      .bg(rgb(if active { SURFACE_ACTIVE } else { BG }))
      .px_3()
      .py_2()
      .flex()
      .items_center()
      .justify_between()
      .child(
         div()
            .font_family(FONT_MONO)
            .text_size(px(9.0))
            .text_color(rgb(TEXT_DIM))
            .child(label.to_string()),
      )
      .child(
         div()
            .font_family(FONT_MONO)
            .text_size(px(11.0))
            .text_color(rgb(if active { PHOSPHOR } else { TEXT_DIM }))
            .overflow_hidden()
            .text_ellipsis()
            .child(value.into()),
      )
}

fn data_block(value: impl Into<String>, active: bool) -> impl IntoElement {
   div()
      .min_h(px(54.0))
      .border_1()
      .border_color(rgb(if active { BORDER_ACTIVE } else { BORDER }))
      .bg(rgb(if active { SURFACE_ACTIVE } else { BG }))
      .p_3()
      .font_family(FONT_MONO)
      .text_size(px(10.0))
      .text_color(rgb(if active { PHOSPHOR } else { TEXT_DIM }))
      .child(value.into())
}

fn led(label: &str, active: bool, color: u32) -> impl IntoElement {
   div()
      .flex()
      .items_center()
      .gap_1()
      .child(
         div()
            .w(px(8.0))
            .h(px(8.0))
            .bg(rgb(if active { color } else { BORDER }))
            .border_1()
            .border_color(rgb(if active { color } else { BORDER })),
      )
      .child(
         div()
            .font_family(FONT_MONO)
            .text_size(px(8.0))
            .text_color(rgb(if active { color } else { TEXT_DIM }))
            .child(label.to_string()),
      )
}

fn level_bar(label: &str, active: bool, color: u32) -> impl IntoElement {
   div()
      .flex()
      .items_center()
      .gap_1()
      .child(
         div()
            .font_family(FONT_MONO)
            .text_size(px(8.0))
            .text_color(rgb(TEXT_DIM))
            .child(label.to_string()),
      )
      .child(
         div()
            .w(px(28.0))
            .h(px(4.0))
            .bg(rgb(if active { color } else { BORDER })),
      )
}

fn readout(value: &str, label: &str) -> impl IntoElement {
   div()
      .flex()
      .flex_col()
      .items_start()
      .gap_1()
      .child(
         div()
            .border_1()
            .border_color(rgb(PHOSPHOR_DIM))
            .bg(rgb(BG))
            .px_3()
            .py_1()
            .font_family(FONT_MONO)
            .text_size(px(40.0))
            .text_color(rgb(PHOSPHOR))
            .child(value.to_string()),
      )
      .child(
         div()
            .font_family(FONT_MONO)
            .text_size(px(9.0))
            .text_color(rgb(TEXT_DIM))
            .child(label.to_string()),
      )
}

fn spectrum_analyzer() -> impl IntoElement {
   div()
      .w(px(220.0))
      .h(px(56.0))
      .border_1()
      .border_color(rgb(BORDER))
      .bg(rgb(BG))
      .flex()
      .flex_col()
      .child(
         div()
            .flex_1()
            .flex()
            .items_end()
            .justify_center()
            .gap_1()
            .px_2()
            .pt_2()
            .child(tach_bar(8, false))
            .child(tach_bar(14, false))
            .child(tach_bar(22, true))
            .child(tach_bar(32, true))
            .child(tach_bar(42, true))
            .child(tach_bar(38, true))
            .child(tach_bar(28, true))
            .child(tach_bar(18, false))
            .child(tach_bar(10, false))
            .child(tach_bar(6, false))
            .child(tach_bar(12, true))
            .child(tach_bar(20, true)),
      )
      .child(div().h(px(1.0)).bg(rgb(PHOSPHOR_DIM)))
}

fn tach_bar(height: i32, active: bool) -> impl IntoElement {
   div()
      .w(px(10.0))
      .h(px(height as f32))
      .bg(rgb(if active { PHOSPHOR_DIM } else { PHOSPHOR_DARK }))
}

fn log_color(line: &str) -> gpui::Rgba {
   if line.starts_with("ERR") {
      rgb(ALERT)
   } else if line.starts_with("OK") {
      rgb(PHOSPHOR)
   } else if line.starts_with('$') {
      rgb(TEXT)
   } else {
      rgb(TEXT_DIM)
   }
}

fn device_status(app: &BeaconApp) -> String {
   match app.status {
      AppStatus::Detecting => "SCANNING USB".to_string(),
      AppStatus::Identifying => "IDENTIFYING".to_string(),
      AppStatus::Error(AppErrorKind::NoDevice) => "NO DEVICE".to_string(),
      AppStatus::Error(AppErrorKind::MultipleDevicesNoSelection) => {
         "MULTIPLE DEVICES - SELECT ONE".to_string()
      },
      _ if app.selected_device_index.is_some() => "DEVICE SELECTED".to_string(),
      _ => "WAITING FOR SCAN".to_string(),
   }
}

fn upload_status(app: &BeaconApp) -> &'static str {
   if app.can_upload() {
      "ARMED"
   } else if app.tycmd.is_none() {
      "TYCMD MISSING"
   } else if app.selected_hex.is_none() {
      "SELECT HEX"
   } else if app.selected_device_index.is_none() {
      "SELECT DEVICE"
   } else {
      "BUSY"
   }
}

fn summarize(output: &str) -> String {
   let summary = output
      .lines()
      .filter(|line| !line.trim().is_empty())
      .take(4)
      .collect::<Vec<_>>()
      .join("\n");

   if summary.is_empty() {
      "IDENTIFY COMPLETE".to_string()
   } else {
      summary
   }
}

fn recovery_message(status: &AppStatus) -> Option<String> {
   match status {
      AppStatus::Error(AppErrorKind::MissingTycmd(path)) => {
         Some(format!(
            "TYCMD SIDECAR MISSING\nExpected:\n{}",
            path.display()
         ))
      },
      AppStatus::Error(AppErrorKind::InvalidHexFile(path)) => {
         Some(format!(
            "INVALID HEX FILE\nSelected path is not a .hex firmware file:\n{}",
            path.display()
         ))
      },
      AppStatus::Error(AppErrorKind::NoDevice)
      | AppStatus::Error(AppErrorKind::CommandFailed { .. }) => {
         Some(
            "NO DEVICE / UPLOAD FAULT\n1. Confirm the Teensy is connected over USB.\n2. Press the \
             physical Program button on the Teensy.\n3. Disconnect extra Teensy boards if more \
             than one is attached.\n4. Unplug and reconnect USB.\n5. Press SCAN, then UPLOAD \
             again."
               .to_string(),
         )
      },
      AppStatus::Error(AppErrorKind::MultipleDevicesNoSelection) => {
         Some(
            "MULTIPLE TEENSYS DETECTED\nSelect one device before upload. If upload targeting is \
             uncertain, disconnect extras and scan again."
               .to_string(),
         )
      },
      AppStatus::Error(AppErrorKind::Io(message)) => Some(format!("I/O FAULT\n{message}")),
      _ => None,
   }
}
