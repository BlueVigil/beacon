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

impl Render for BeaconApp {
   fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
      div()
         .size_full()
         .bg(rgb(0x0F1115))
         .flex()
         .items_center()
         .justify_center()
         .child(
            div()
               .flex()
               .flex_col()
               .items_center()
               .gap_3()
               .child(
                  div()
                     .w(px(72.0))
                     .h(px(72.0))
                     .rounded_full()
                     .border_1()
                     .border_color(rgb(0x6EE7B7))
                     .bg(rgb(0x17211F))
                     .shadow_lg()
                     .flex()
                     .items_center()
                     .justify_center()
                     .child(
                        div()
                           .w(px(18.0))
                           .h(px(18.0))
                           .rounded_full()
                           .bg(rgb(0x6EE7B7)),
                     ),
               )
               .child(
                  div()
                     .text_size(px(44.0))
                     .text_color(rgb(0xF8FAFC))
                     .font_weight(gpui::FontWeight::SEMIBOLD)
                     .child("Beacon"),
               )
               .child(
                  div()
                     .text_size(px(14.0))
                     .text_color(rgb(0x94A3B8))
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
