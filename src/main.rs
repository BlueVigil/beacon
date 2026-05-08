use gpui::{
   App,
   Application,
   Bounds,
   Context,
   Render,
   Window,
   WindowBounds,
   WindowOptions,
   div,
   prelude::*,
   px,
   rgb,
   size,
};

struct BeaconApp;

const BLACK: u32 = 0x000000;
const AMBER: u32 = 0xFF8200;
const AMBER_DARK: u32 = 0x241100;
const WHITE: u32 = 0xFFFFFF;
const MUTED_WHITE: u32 = 0xB8B8B8;

impl Render for BeaconApp {
   fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
      div()
         .size_full()
         .bg(rgb(BLACK))
         .flex()
         .items_center()
         .justify_center()
         .child(
            div()
               .flex()
               .flex_col()
               .items_center()
               .gap_4()
               .child(
                  div()
                     .w(px(72.0))
                     .h(px(72.0))
                     .rounded_full()
                     .border_1()
                     .border_color(rgb(AMBER))
                     .bg(rgb(AMBER_DARK))
                     .shadow_lg()
                     .flex()
                     .items_center()
                     .justify_center()
                     .child(div().w(px(18.0)).h(px(18.0)).rounded_full().bg(rgb(AMBER))),
               )
               .child(
                  div()
                     .text_size(px(48.0))
                     .text_color(rgb(WHITE))
                     .font_weight(gpui::FontWeight::SEMIBOLD)
                     .child("Beacon"),
               )
               .child(
                  div()
                     .text_size(px(14.0))
                     .text_color(rgb(MUTED_WHITE))
                     .child("Teensy firmware uploader"),
               ),
         )
   }
}

fn main() {
   Application::new().run(|cx: &mut App| {
      let bounds = Bounds::centered(None, size(px(860.0), px(560.0)), cx);

      cx.open_window(
         WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
         },
         |_, cx| cx.new(|_| BeaconApp),
      )
      .unwrap();
   });
}
