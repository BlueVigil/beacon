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
               .w_full()
               .max_w(px(1240.0))
               .min_w(px(860.0))
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
                     .min_h(px(0.0))
                     .child(
                        div()
                           .flex_1()
                           .flex_basis(px(390.0))
                           .flex()
                           .flex_col()
                           .gap_3()
                           .child(self.firmware_panel(cx))
                           .child(self.device_panel(cx)),
                     )
                     .child(
                        div()
                           .flex_1()
                           .flex_basis(px(460.0))
                           .flex()
                           .flex_col()
                           .gap_3()
                           .child(self.upload_panel(cx))
                           .child(self.output_panel()),
                     ),
               ),
         )
   }
}

const FONT_MONO: &str = "Courier New";
const FONT_TITLE: &str = "Menlo";

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
            div()
               .flex()
               .items_center()
               .justify_between()
               .px_4()
               .py_2()
               .child(
                  div()
                     .flex()
                     .flex_col()
                     .gap_1()
                     .child(
                        div()
                           .font_family(FONT_TITLE)
                           .text_size(px(18.0))
                           .text_color(rgb(PHOSPHOR))
                           .font_weight(FontWeight::BOLD)
                           .child("BEACON"),
                     )
                     .child(header_status(self)),
               )
               .child(
                  div()
                     .flex()
                     .items_center()
                     .gap_2()
                     .child(step_indicator(
                        "File selected",
                        self.selected_hex.is_some(),
                        AMBER,
                     ))
                     .child(step_chevron(self.selected_device_index.is_some()))
                     .child(step_indicator(
                        "Teensy selected",
                        self.selected_device_index.is_some(),
                        PHOSPHOR,
                     ))
                     .child(step_chevron(self.can_upload()))
                     .child(step_indicator("Ready", self.can_upload(), PHOSPHOR)),
               ),
         )
         .child(
            div()
               .flex_1()
               .flex()
               .items_center()
               .justify_center()
               .px_4()
               .child(spectrum_analyzer()),
         )
         .child(
            div()
               .flex()
               .items_center()
               .px_4()
               .py_2()
               .border_t_1()
               .border_color(rgb(BORDER)),
         )
   }

   fn firmware_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
      let identify = self
         .identify_output
         .as_ref()
         .map(|o| summarize(o))
         .unwrap_or_else(|| {
            if self.selected_hex.is_some() {
               "CHECK PENDING".to_string()
            } else {
               "NO FIRMWARE INFO".to_string()
            }
         });

      panel("[01]", "FIRMWARE", self.selected_hex.is_some(), AMBER)
         .child(data_field(
            "FILE",
            self
               .selected_hex
               .as_ref()
               .map(|p| p.display().to_string())
               .unwrap_or_else(|| "NO HEX SELECTED".into()),
            self.selected_hex.is_some(),
         ))
         .child(data_block(identify, self.selected_hex.is_some()))
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
         "[02]",
         "DEVICE",
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

   fn upload_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
      let can_upload = self.can_upload();
      panel("[04]", "UPLOAD", can_upload, AMBER)
         .child(data_field("STATUS", upload_status(self), can_upload))
         .child(action_button(
            "EXECUTE UPLOAD",
            !can_upload,
            cx.listener(Self::upload),
         ))
   }

   fn output_panel(&self) -> impl IntoElement {
      let lines = if self.output_lines.is_empty() {
         vec!["Waiting for commands".to_string()]
      } else {
         self.output_lines.iter().rev().take(12).cloned().collect()
      };

      panel("[05]", "OUTPUT", !self.output_lines.is_empty(), PHOSPHOR).child(
         div()
            .flex_1()
            .min_h(px(160.0))
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
}

fn panel(index: &'static str, title: &'static str, active: bool, accent: u32) -> gpui::Div {
   div()
      .border_1()
      .border_color(rgb(BORDER))
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
            .border_color(rgb(BORDER))
            .child(
               div()
                  .w(px(6.0))
                  .h(px(6.0))
                  .bg(rgb(if active { accent } else { BORDER })),
            )
            .child(
               div()
                  .font_family(FONT_MONO)
                  .text_size(px(9.0))
                  .text_color(rgb(TEXT_DIM))
                  .child(index),
            )
            .child(
               div()
                  .font_family(FONT_MONO)
                  .text_size(px(10.0))
                  .text_color(rgb(TEXT))
                  .font_weight(FontWeight::SEMIBOLD)
                  .child(title),
            ),
      )
}

fn header_status(app: &BeaconApp) -> impl IntoElement {
   let error_state = matches!(app.status, AppStatus::Error(_));

   div()
      .font_family(FONT_MONO)
      .text_size(px(10.0))
      .text_color(rgb(if error_state { ALERT } else { PHOSPHOR }))
      .child(status_sentence(app))
}

fn status_sentence(app: &BeaconApp) -> &'static str {
   match app.status {
      AppStatus::Idle => "Choose a firmware file or scan for a Teensy.",
      AppStatus::SelectingFile => "Choosing firmware file.",
      AppStatus::Detecting => "Scanning for connected Teensy boards.",
      AppStatus::Identifying => "Checking firmware file.",
      AppStatus::Ready => "Ready to upload.",
      AppStatus::Uploading => "Uploading firmware.",
      AppStatus::Success => "Upload complete.",
      AppStatus::Error(AppErrorKind::MissingTycmd(_)) => "Bundled tycmd is missing.",
      AppStatus::Error(AppErrorKind::InvalidHexFile(_)) => "Selected file is not a .hex file.",
      AppStatus::Error(AppErrorKind::NoDevice) => "No Teensy detected.",
      AppStatus::Error(AppErrorKind::MultipleDevicesNoSelection) => "Select one Teensy.",
      AppStatus::Error(AppErrorKind::CommandFailed { .. }) => "Command failed.",
      AppStatus::Error(AppErrorKind::Io(_)) => "I/O error.",
   }
}

fn step_indicator(label: &str, active: bool, color: u32) -> impl IntoElement {
   div()
      .flex()
      .items_center()
      .gap_1()
      .child(status_dot(active, color))
      .child(
         div()
            .font_family(FONT_MONO)
            .text_size(px(9.0))
            .text_color(rgb(if active { TEXT } else { TEXT_DIM }))
            .child(label.to_string()),
      )
}

fn step_chevron(active: bool) -> impl IntoElement {
   div()
      .font_family(FONT_MONO)
      .text_size(px(13.0))
      .text_color(rgb(if active { PHOSPHOR_DIM } else { BORDER }))
      .child(">")
}

fn status_dot(active: bool, color: u32) -> impl IntoElement {
   div()
      .w(px(7.0))
      .h(px(7.0))
      .bg(rgb(if active { color } else { BORDER }))
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
      .border_color(rgb(if disabled { BORDER } else { BORDER_ACTIVE }))
      .bg(rgb(if disabled { SURFACE } else { SURFACE_ACTIVE }))
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
            .bg(rgb(if disabled { BORDER } else { PHOSPHOR_DIM })),
      )
      .child(
         div()
            .text_size(px(11.0))
            .text_color(rgb(if disabled { TEXT_DIM } else { TEXT }))
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
      AppStatus::Identifying => "CHECKING FIRMWARE".to_string(),
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
