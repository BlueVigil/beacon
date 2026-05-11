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
   SharedString,
   WindowBounds,
   WindowOptions,
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
         let bounds = Bounds::centered(None, size(px(960.0), px(640.0)), cx);

         cx.open_window(
            WindowOptions {
               window_bounds: Some(WindowBounds::Windowed(bounds)),
               window_min_size: Some(size(px(900.0), px(600.0))),
               ..Default::default()
            },
            |_, cx| cx.new(BeaconApp::new),
         )
         .unwrap();
      });
}
