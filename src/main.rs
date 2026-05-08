mod app;
mod theme;
mod tycmd;
mod ui;

use app::BeaconApp;
use gpui::{
   App,
   AppContext,
   Application,
   Bounds,
   WindowBounds,
   WindowOptions,
   px,
   size,
};

fn main() {
   Application::new().run(|cx: &mut App| {
      let bounds = Bounds::centered(None, size(px(860.0), px(560.0)), cx);

      cx.open_window(
         WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
         },
         |_, cx| cx.new(BeaconApp::new),
      )
      .unwrap();
   });
}
