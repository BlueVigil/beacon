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
   theme::{
      BLACK,
      GREEN,
      GREEN_DARK,
      GREEN_DIM,
      MUTED_WHITE,
      PANEL,
      PANEL_LINE,
      RED,
      RED_DARK,
      WHITE,
   },
};

impl Render for BeaconApp {
   fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
      div()
         .size_full()
         .bg(rgb(BLACK))
         .flex()
         .justify_center()
         .p_5()
         .child(
            div()
               .w(px(820.0))
               .h_full()
               .flex()
               .flex_col()
               .gap_4()
               .child(self.instrument_cluster())
               .child(
                  div()
                     .flex()
                     .gap_4()
                     .flex_1()
                     .child(
                        div()
                           .w(px(380.0))
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

impl BeaconApp {
   fn instrument_cluster(&self) -> impl IntoElement {
      div()
         .h(px(156.0))
         .rounded(px(22.0))
         .border_1()
         .border_color(rgb(PANEL_LINE))
         .bg(rgb(PANEL))
         .shadow_lg()
         .flex()
         .flex_col()
         .justify_between()
         .p_4()
         .child(
            div()
               .flex()
               .items_center()
               .justify_between()
               .child(
                  div()
                     .text_size(px(12.0))
                     .text_color(status_color(&self.status))
                     .font_weight(FontWeight::SEMIBOLD)
                     .child(self.status_text()),
               )
               .child(
                  div()
                     .flex()
                     .gap_1()
                     .child(light(GREEN))
                     .child(light(if self.selected_hex.is_some() {
                        GREEN
                     } else {
                        GREEN_DIM
                     }))
                     .child(light(if matches!(self.status, AppStatus::Error(_)) {
                        RED
                     } else {
                        RED_DARK
                     }))
                     .child(light(if self.selected_device_index.is_some() {
                        GREEN
                     } else {
                        GREEN_DIM
                     })),
               ),
         )
         .child(
            div()
               .flex()
               .items_end()
               .justify_between()
               .child(readout(
                  &format!("{:02}", self.devices.len().min(99)),
                  "DEVICE MPH",
               ))
               .child(tach_bar())
               .child(readout(
                  if self.selected_hex.is_some() {
                     "HX"
                  } else {
                     "BE"
                  },
                  "FIRMWARE",
               )),
         )
         .child(
            div()
               .flex()
               .items_center()
               .justify_between()
               .child(
                  div()
                     .flex()
                     .gap_1()
                     .child(bar(self.selected_hex.is_some()))
                     .child(bar(self.selected_device_index.is_some()))
                     .child(bar(matches!(
                        self.status,
                        AppStatus::Ready | AppStatus::Success
                     )))
                     .child(bar(matches!(self.status, AppStatus::Uploading))),
               )
               .child(
                  div()
                     .text_size(px(38.0))
                     .text_color(rgb(WHITE))
                     .font_weight(FontWeight::SEMIBOLD)
                     .child("Beacon"),
               )
               .child(
                  div()
                     .text_size(px(12.0))
                     .text_color(rgb(MUTED_WHITE))
                     .child("Teensy firmware uploader"),
               ),
         )
   }

   fn firmware_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
      section("FIRMWARE")
         .child(value_line(
            self
               .selected_hex
               .as_ref()
               .map(|path| path.display().to_string())
               .unwrap_or_else(|| "NO HEX SELECTED".to_string()),
            self.selected_hex.is_some(),
         ))
         .child(dashboard_button(
            "CHOOSE HEX",
            self.is_busy(),
            cx.listener(Self::choose_hex),
         ))
   }

   fn device_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
      let selected = self
         .selected_device_index
         .and_then(|index| self.devices.get(index))
         .map(|device| device.label.as_str())
         .unwrap_or("NO TEENSY SELECTED");

      let mut panel = section("DEVICE")
         .child(value_line(
            device_status(self),
            self.selected_device_index.is_some(),
         ))
         .child(value_line(selected, self.selected_device_index.is_some()))
         .child(dashboard_button(
            "SCAN",
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
               .border_color(rgb(if selected { GREEN } else { GREEN_DIM }))
               .bg(rgb(if selected { GREEN_DARK } else { PANEL }))
               .px_3()
               .py_2()
               .text_size(px(11.0))
               .text_color(rgb(if selected { GREEN } else { MUTED_WHITE }))
               .overflow_hidden()
               .text_ellipsis()
               .child(device.label.clone())
               .on_click(cx.listener(move |this, _event, window, cx| {
                  this.select_device(index, window, cx);
               }))
         });

         panel = panel.child(div().flex().flex_col().gap_2().children(rows));
      }

      panel
   }

   fn verify_panel(&self) -> impl IntoElement {
      let identify = self
         .identify_output
         .as_ref()
         .map(|output| summarize(output))
         .unwrap_or_else(|| "IDENTIFY PENDING".to_string());

      section("VERIFY").child(value_block(identify, self.identify_output.is_some()))
   }

   fn upload_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
      section("UPLOAD")
         .child(value_line(upload_status(self), self.can_upload()))
         .child(dashboard_button(
            "UPLOAD",
            !self.can_upload(),
            cx.listener(Self::upload),
         ))
   }

   fn output_panel(&self) -> impl IntoElement {
      let lines = if self.output_lines.is_empty() {
         vec!["INFO waiting for command output".to_string()]
      } else {
         self.output_lines.iter().rev().take(10).cloned().collect()
      };

      section("OUTPUT").child(
         div()
            .h(px(164.0))
            .border_1()
            .border_color(rgb(GREEN_DIM))
            .bg(rgb(0x000D05))
            .p_3()
            .overflow_hidden()
            .flex()
            .flex_col()
            .gap_1()
            .children(lines.into_iter().rev().map(|line| {
               div()
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

      section("RECOVERY / STATUS").child(value_block(
         message.unwrap_or_else(|| {
            match self.status {
               AppStatus::Success => "UPLOAD COMPLETE".to_string(),
               AppStatus::Ready => "READY TO UPLOAD".to_string(),
               AppStatus::Uploading => "UPLOAD IN PROGRESS".to_string(),
               _ => "WAITING".to_string(),
            }
         }),
         active || matches!(self.status, AppStatus::Success | AppStatus::Ready),
      ))
   }
}

fn section(title: &'static str) -> gpui::Div {
   div()
      .border_1()
      .border_color(rgb(PANEL_LINE))
      .bg(rgb(PANEL))
      .p_3()
      .flex()
      .flex_col()
      .gap_2()
      .child(
         div()
            .text_size(px(11.0))
            .text_color(rgb(GREEN))
            .font_weight(FontWeight::SEMIBOLD)
            .child(title),
      )
}

fn dashboard_button(
   label: &'static str,
   disabled: bool,
   listener: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
   div()
      .id(SharedString::from(format!("button-{label}")))
      .cursor_pointer()
      .border_1()
      .border_color(rgb(if disabled { GREEN_DIM } else { GREEN }))
      .bg(rgb(if disabled { PANEL } else { GREEN_DARK }))
      .px_3()
      .py_2()
      .opacity(if disabled { 0.45 } else { 1.0 })
      .text_size(px(12.0))
      .text_color(rgb(if disabled { GREEN_DIM } else { GREEN }))
      .font_weight(FontWeight::SEMIBOLD)
      .child(label)
      .when(!disabled, |button| button.on_click(listener))
}

fn value_line(value: impl Into<String>, active: bool) -> impl IntoElement {
   div()
      .border_1()
      .border_color(rgb(if active { GREEN_DIM } else { PANEL_LINE }))
      .bg(rgb(if active { GREEN_DARK } else { BLACK }))
      .px_3()
      .py_2()
      .text_size(px(11.0))
      .text_color(rgb(if active { GREEN } else { MUTED_WHITE }))
      .overflow_hidden()
      .text_ellipsis()
      .child(value.into())
}

fn value_block(value: impl Into<String>, active: bool) -> impl IntoElement {
   div()
      .min_h(px(58.0))
      .border_1()
      .border_color(rgb(if active { GREEN_DIM } else { PANEL_LINE }))
      .bg(rgb(if active { GREEN_DARK } else { BLACK }))
      .p_3()
      .text_size(px(11.0))
      .text_color(rgb(if active { GREEN } else { MUTED_WHITE }))
      .child(value.into())
}

fn light(color: u32) -> impl IntoElement {
   div().w(px(8.0)).h(px(8.0)).bg(rgb(color))
}

fn bar(active: bool) -> impl IntoElement {
   div()
      .w(px(32.0))
      .h(px(5.0))
      .bg(rgb(if active { GREEN } else { GREEN_DIM }))
}

fn readout(value: &str, label: &'static str) -> impl IntoElement {
   div()
      .flex()
      .flex_col()
      .gap_1()
      .child(
         div()
            .text_size(px(46.0))
            .text_color(rgb(GREEN))
            .font_weight(FontWeight::SEMIBOLD)
            .child(value.to_string()),
      )
      .child(
         div()
            .text_size(px(10.0))
            .text_color(rgb(GREEN_DIM))
            .child(label),
      )
}

fn tach_bar() -> impl IntoElement {
   div()
      .w(px(210.0))
      .h(px(54.0))
      .border_1()
      .border_color(rgb(GREEN_DIM))
      .bg(rgb(GREEN_DARK))
      .flex()
      .items_end()
      .gap_1()
      .p_2()
      .child(tach_column(12, false))
      .child(tach_column(18, false))
      .child(tach_column(26, true))
      .child(tach_column(34, true))
      .child(tach_column(44, true))
      .child(tach_column(48, true))
      .child(tach_column(36, true))
      .child(tach_column(22, false))
}

fn tach_column(height: i32, active: bool) -> impl IntoElement {
   div()
      .w(px(12.0))
      .h(px(height as f32))
      .bg(rgb(if active { GREEN } else { GREEN_DIM }))
}

fn status_color(status: &AppStatus) -> gpui::Rgba {
   rgb(if matches!(status, AppStatus::Error(_)) {
      RED
   } else {
      GREEN
   })
}

fn log_color(line: &str) -> gpui::Rgba {
   if line.starts_with("ERR") {
      rgb(RED)
   } else if line.starts_with("OK") {
      rgb(GREEN)
   } else if line.starts_with('$') {
      rgb(WHITE)
   } else {
      rgb(MUTED_WHITE)
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
