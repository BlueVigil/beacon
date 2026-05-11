use gpui::{
   App,
   Context,
   FontWeight,
   IntoElement,
   Render,
   SharedString,
   Window,
   div,
   prelude::*,
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
                           .child(self.firmware_panel(cx))
                           .child(workflow_connector(device_module_status(self)))
                           .child(self.device_panel(cx))
                           .child(workflow_connector(upload_module_status(self)))
                           .child(self.upload_panel(cx)),
                     )
                     .child(
                        div()
                           .flex_1()
                           .flex_basis(px(460.0))
                           .flex()
                           .flex_col()
                           .child(self.output_panel()),
                     ),
               ),
         )
   }
}

const FONT_MONO: &str = "Courier New";

struct OutputLineDisplay {
   line:            String,
   is_latest_block: bool,
}
const FONT_TITLE: &str = "Menlo";

#[derive(Clone, Copy)]
enum ModuleStatus {
   Pending,
   Next,
   Done,
}

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
                  div().flex().flex_col().gap_1().child(
                     div()
                        .font_family(FONT_TITLE)
                        .text_size(px(18.0))
                        .text_color(rgb(PHOSPHOR))
                        .font_weight(FontWeight::BOLD)
                        .child("BEACON"),
                  ),
               )
               .child(
                  div()
                     .flex()
                     .items_center()
                     .gap_2()
                     .child(step_indicator(
                        "File selected",
                        firmware_module_status(self),
                     ))
                     .child(step_chevron(device_module_status(self)))
                     .child(step_indicator(
                        "Teensy selected",
                        device_module_status(self),
                     ))
                     .child(step_chevron(upload_module_status(self)))
                     .child(step_indicator("Ready", upload_module_status(self))),
               ),
         )
         .child(
            div()
               .flex_1()
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

      panel("[01]", "FIRMWARE", firmware_module_status(self))
         .flex_none()
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

      let mut p = panel("[02]", "DEVICE", device_module_status(self))
         .flex_none()
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
      panel("[03]", "UPLOAD", upload_module_status(self))
         .flex_none()
         .child(data_field("STATUS", upload_status(self), can_upload))
         .child(action_button(
            "EXECUTE UPLOAD",
            !can_upload,
            cx.listener(Self::upload),
         ))
   }

   fn output_panel(&self) -> impl IntoElement {
      let lines = output_lines_for_display(&self.output_lines);

      panel("[04]", "OUTPUT", output_module_status(self))
         .flex_1()
         .min_h(px(0.0))
         .child(
            div()
               .flex_1()
               .min_h(px(0.0))
               .border_1()
               .border_color(rgb(BORDER))
               .bg(rgb(BG))
               .p_3()
               .map(|mut viewport| {
                  viewport.interactivity().base_style.overflow.x = Some(gpui::Overflow::Scroll);
                  viewport.interactivity().base_style.overflow.y = Some(gpui::Overflow::Scroll);
                  viewport.interactivity().base_style.scrollbar_width = Some(px(8.0).into());
                  viewport
               })
               .flex()
               .flex_col()
               .justify_end()
               .child(
                  div()
                     .flex_none()
                     .flex()
                     .flex_col()
                     .gap_1()
                     .children(lines.into_iter().map(|entry| {
                        div()
                           .flex_none()
                           .font_family(FONT_MONO)
                           .text_size(px(10.0))
                           .line_height(px(14.0))
                           .text_color(log_color(&entry.line, entry.is_latest_block))
                           .overflow_hidden()
                           .text_ellipsis()
                           .child(entry.line)
                     })),
               ),
         )
   }
}

fn panel(index: &'static str, title: &'static str, status: ModuleStatus) -> gpui::Div {
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
                  .bg(rgb(module_status_color(status))),
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

fn firmware_module_status(app: &BeaconApp) -> ModuleStatus {
   if app.selected_hex.is_some() {
      ModuleStatus::Done
   } else {
      ModuleStatus::Next
   }
}

fn device_module_status(app: &BeaconApp) -> ModuleStatus {
   if app.selected_device_index.is_some() {
      ModuleStatus::Done
   } else if app.selected_hex.is_some() {
      ModuleStatus::Next
   } else {
      ModuleStatus::Pending
   }
}

fn upload_module_status(app: &BeaconApp) -> ModuleStatus {
   if matches!(app.status, AppStatus::Success) {
      ModuleStatus::Done
   } else if app.can_upload() {
      ModuleStatus::Next
   } else {
      ModuleStatus::Pending
   }
}

fn output_module_status(app: &BeaconApp) -> ModuleStatus {
   if app.output_lines.is_empty() {
      ModuleStatus::Pending
   } else {
      ModuleStatus::Done
   }
}

fn module_status_color(status: ModuleStatus) -> u32 {
   match status {
      ModuleStatus::Pending => PHOSPHOR_DARK,
      ModuleStatus::Next => AMBER,
      ModuleStatus::Done => PHOSPHOR,
   }
}

fn workflow_connector(status: ModuleStatus) -> impl IntoElement {
   div()
      .flex_1()
      .min_h(px(32.0))
      .flex()
      .items_center()
      .justify_center()
      .child(
         div()
            .flex()
            .flex_col()
            .items_center()
            .gap_1()
            .children((0..5).map(|_| {
               div()
                  .font_family(FONT_MONO)
                  .text_size(px(13.0))
                  .line_height(px(10.0))
                  .text_color(rgb(module_status_color(status)))
                  .child("⌄")
            })),
      )
}

fn step_indicator(label: &str, status: ModuleStatus) -> impl IntoElement {
   let done = matches!(status, ModuleStatus::Done);

   div()
      .flex()
      .items_center()
      .gap_1()
      .child(status_dot(status))
      .child(
         div()
            .font_family(FONT_MONO)
            .text_size(px(9.0))
            .text_color(rgb(if done { TEXT } else { TEXT_DIM }))
            .child(label.to_string()),
      )
}

fn step_chevron(next_status: ModuleStatus) -> impl IntoElement {
   let active = matches!(next_status, ModuleStatus::Next | ModuleStatus::Done);

   div()
      .font_family(FONT_MONO)
      .text_size(px(13.0))
      .text_color(rgb(if active { PHOSPHOR_DIM } else { BORDER }))
      .child("›")
}

fn status_dot(status: ModuleStatus) -> impl IntoElement {
   div()
      .w(px(7.0))
      .h(px(7.0))
      .bg(rgb(module_status_color(status)))
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

fn output_lines_for_display(lines: &[String]) -> Vec<OutputLineDisplay> {
   if lines.is_empty() {
      return vec![OutputLineDisplay {
         line:            "Waiting for commands".to_string(),
         is_latest_block: true,
      }];
   }

   let latest_command_index = lines.iter().rposition(|line| line.starts_with("$ "));
   lines
      .iter()
      .enumerate()
      .map(|(offset, line)| {
         let is_latest_block = latest_command_index.is_none_or(|index| offset >= index);

         OutputLineDisplay {
            line: line.clone(),
            is_latest_block,
         }
      })
      .collect()
}

fn log_color(line: &str, is_latest_block: bool) -> gpui::Rgba {
   if is_latest_block {
      active_log_color(line)
   } else {
      stale_log_color(line)
   }
}

fn active_log_color(line: &str) -> gpui::Rgba {
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

fn stale_log_color(line: &str) -> gpui::Rgba {
   if line.starts_with("ERR") {
      rgb(0x8A4545)
   } else if line.starts_with("OK") {
      rgb(PHOSPHOR_DIM)
   } else if line.starts_with('$') {
      rgb(TEXT_DIM)
   } else {
      rgb(0x384838)
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
