mod actions;
mod app;
mod theme;
mod tycmd;
mod ui;

use std::{
   borrow::Cow,
   fs,
   path::PathBuf,
};

use anyhow::Result;
use app::BeaconApp;
use gpui::{
   App,
   AppContext,
   Application,
   AssetSource,
   Bounds,
   Focusable,
   KeyBinding,
   SharedString,
   TitlebarOptions,
   WindowBounds,
   WindowDecorations,
   WindowOptions,
   point,
   px,
   size,
};

struct Assets {
   base: PathBuf,
}

impl AssetSource for Assets {
   fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
      fs::read(self.base.join(path))
         .map(|data| Some(Cow::Owned(data)))
         .map_err(Into::into)
   }

   fn list(&self, path: &str) -> Result<Vec<SharedString>> {
      fs::read_dir(self.base.join(path))
         .map(|entries| {
            entries
               .filter_map(|entry| {
                  entry
                     .ok()
                     .and_then(|entry| entry.file_name().into_string().ok())
                     .map(SharedString::from)
               })
               .collect()
         })
         .map_err(Into::into)
   }
}

fn main() {
   Application::new()
      .with_assets(Assets {
         base: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"),
      })
      .run(|cx: &mut App| {
         cx.bind_keys([
            KeyBinding::new("cmd-o", actions::LoadHex, Some("Beacon")),
            KeyBinding::new("cmd-shift-r", actions::ScanUsb, Some("Beacon")),
            KeyBinding::new("cmd-u", actions::Upload, Some("Beacon")),
            KeyBinding::new("cmd-r", actions::CycleAutoMode, Some("Beacon")),
         ]);

         let bounds = Bounds::centered(None, size(px(1080.0), px(900.0)), cx);

         cx.open_window(
            WindowOptions {
               window_bounds: Some(WindowBounds::Windowed(bounds)),
               window_min_size: Some(size(px(900.0), px(840.0))),
               titlebar: Some(TitlebarOptions {
                  appears_transparent: true,
                  traffic_light_position: Some(point(px(12.0), px(10.0))),
                  ..Default::default()
               }),
               window_decorations: Some(WindowDecorations::Client),
               ..Default::default()
            },
            |window, cx| {
               let app = cx.new(BeaconApp::new);
               window.focus(&app.focus_handle(cx));
               app
            },
         )
         .unwrap();
      });
}
