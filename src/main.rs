mod actions;
mod app;
mod theme;
mod tycmd;
mod ui;

use std::{
   borrow::Cow,
   fs,
   io,
   path::PathBuf,
   sync::mpsc,
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
   Menu,
   MenuItem,
   SharedString,
   SystemMenuType,
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
   let (open_urls_sender, open_urls_receiver) = mpsc::channel();
   let application = Application::new().with_assets(Assets { base: asset_root() });

   application.on_open_urls({
      let open_urls_sender = open_urls_sender.clone();
      move |urls| {
         let _ = open_urls_sender.send(urls);
      }
   });

   application.run(move |cx: &mut App| {
      cx.bind_keys([
         KeyBinding::new("cmd-o", actions::LoadHex, Some("Beacon")),
         KeyBinding::new("cmd-shift-r", actions::ScanUsb, Some("Beacon")),
         KeyBinding::new("cmd-u", actions::Upload, Some("Beacon")),
         KeyBinding::new("cmd-r", actions::CycleAutoMode, Some("Beacon")),
         KeyBinding::new("cmd-q", actions::Quit, Some("Beacon")),
         KeyBinding::new("cmd-w", actions::Quit, Some("Beacon")),
      ]);
      cx.set_menus(vec![
         Menu {
            name:  "Beacon".into(),
            items: vec![
               MenuItem::os_submenu("Services", SystemMenuType::Services),
               MenuItem::separator(),
               MenuItem::action("Quit Beacon", actions::Quit),
            ],
         },
         Menu {
            name:  "Actions".into(),
            items: vec![
               MenuItem::action("Load Hex", actions::LoadHex),
               MenuItem::action("Scan USB", actions::ScanUsb),
               MenuItem::separator(),
               MenuItem::action("Upload", actions::Upload),
               MenuItem::action("Cycle Auto Mode", actions::CycleAutoMode),
            ],
         },
      ]);
      load_bundled_fonts(cx).unwrap();
      for path in std::env::args().skip(1) {
         let _ = open_urls_sender.send(vec![path]);
      }

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
            let app = cx.new(|cx| BeaconApp::new(cx, open_urls_receiver));
            window.focus(&app.focus_handle(cx));
            app
         },
      )
      .unwrap();
   });
}

fn asset_root() -> PathBuf {
   packaged_resource_root()
      .map(|root| root.join("assets"))
      .filter(|path| path.exists())
      .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"))
}

fn load_bundled_fonts(cx: &mut App) -> Result<()> {
   let fonts_dir = asset_root().join("fonts");
   let fonts = read_font_files(&fonts_dir)?;

   if !fonts.is_empty() {
      cx.text_system().add_fonts(fonts)?;
   }

   Ok(())
}

fn read_font_files(path: &PathBuf) -> Result<Vec<Cow<'static, [u8]>>> {
   let entries = match fs::read_dir(path) {
      Ok(entries) => entries,
      Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
      Err(error) => return Err(error.into()),
   };

   let mut fonts = Vec::new();

   for entry in entries {
      let path = entry?.path();
      if path.is_dir() {
         fonts.extend(read_font_files(&path)?);
         continue;
      }

      let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
         continue;
      };

      if matches!(extension.to_ascii_lowercase().as_str(), "otf" | "ttf") {
         fonts.push(Cow::Owned(fs::read(path)?));
      }
   }

   Ok(fonts)
}

fn packaged_resource_root() -> Option<PathBuf> {
   let executable = std::env::current_exe().ok()?;

   for ancestor in executable.ancestors() {
      if ancestor
         .extension()
         .is_some_and(|extension| extension == "app")
      {
         return Some(ancestor.join("Contents").join("Resources"));
      }
   }

   executable.parent().map(|app_dir| app_dir.join("resources"))
}
