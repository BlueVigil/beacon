// SPDX-License-Identifier: AGPL-3.0-only

use std::{
   path::Path,
   thread,
   time::Duration,
};

use anyhow::{
   Context as _,
   Result,
   bail,
};
use hidapi::{
   HidApi,
   HidDevice,
};

use super::{
   device::{
      identify_hid_model,
      is_teensy_hid,
      is_teensy_seremu,
      teensy_serial_ports,
   },
   firmware::load_firmware_file,
   model::Model,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UploadOptions {
   pub wait:         bool,
   pub no_reset:     bool,
   pub no_check:     bool,
   pub no_rtc:       bool,
   pub rtc_utc:      bool,
   pub delegate:     bool,
   pub board_filter: Option<String>,
}

pub fn upload_firmware(path: &Path, options: &UploadOptions) -> Result<()> {
   let firmware = load_firmware_file(path)?;
   let firmware_models = firmware.identify_models();
   let api = HidApi::new()?;
   let (device, model) = open_or_reboot_bootloader(&api, options)?;

   if !options.no_check && !firmware_models.is_empty() && !firmware_models.contains(&model) {
      bail!(
         "firmware is for {}, but selected device is {}",
         firmware_models
            .iter()
            .map(|model| model.info().name)
            .collect::<Vec<_>>()
            .join(", "),
         model.info().name
      );
   }

   let program = firmware
      .programs
      .first()
      .context("firmware does not contain a program")?;
   let settings = HalfKaySettings::for_model(model)?;

   if program.max_address > settings.max_address as usize {
      bail!("firmware is too big for {}", model.info().name);
   }

   let mut first = true;
   for address in (settings.min_address..program.max_address as u32).step_by(settings.block_size) {
      let mut block = vec![0; settings.block_size];
      let len = program.extract(address, &mut block);
      if len == 0 {
         continue;
      }

      halfkay_send(
         &device,
         settings.version,
         settings.block_size,
         address,
         &block[..len],
         250,
         Duration::from_millis(100),
      )?;

      thread::sleep(if first {
         Duration::from_millis(500)
      } else {
         Duration::from_millis(10)
      });
      first = false;
   }

   if !options.no_reset {
      halfkay_send(
         &device,
         settings.version,
         settings.block_size,
         0x00FF_FFFF,
         &[],
         25,
         Duration::from_millis(20),
      )?;
   }

   Ok(())
}

fn open_or_reboot_bootloader(api: &HidApi, options: &UploadOptions) -> Result<(HidDevice, Model)> {
   if let Some(bootloader) = open_bootloader_once(api, options)? {
      return Ok(bootloader);
   }

   if !options.wait {
      reboot_running_teensy(api, options)?;
   }

   loop {
      let api = HidApi::new()?;
      if let Some(bootloader) = open_bootloader_once(&api, options)? {
         return Ok(bootloader);
      }

      thread::sleep(Duration::from_millis(100));
   }
}

fn open_bootloader_once(
   api: &HidApi,
   options: &UploadOptions,
) -> Result<Option<(HidDevice, Model)>> {
   let mut matches = api.device_list().filter(|info| {
      is_teensy_hid(info)
         && matches_filter(
            info.serial_number(),
            &info.path().to_string_lossy(),
            options,
         )
   });

   let Some(info) = matches.next() else {
      return Ok(None);
   };
   if matches.next().is_some() {
      bail!("multiple Teensy bootloaders found; select a device before upload");
   }

   let model = identify_hid_model(info);
   if model == Model::Teensy {
      bail!("could not identify Teensy bootloader model");
   }

   Ok(Some((info.open_device(api)?, model)))
}

fn reboot_running_teensy(api: &HidApi, options: &UploadOptions) -> Result<()> {
   let mut rebooted = false;
   let mut found = false;

   for info in api.device_list().filter(|info| is_teensy_seremu(info)) {
      if !matches_filter(
         info.serial_number(),
         &info.path().to_string_lossy(),
         options,
      ) {
         continue;
      }
      found = true;
      let device = info.open_device(api)?;
      device.send_feature_report(&[0, 0xA9, 0x45, 0xC2, 0x6B])?;
      rebooted = true;
   }

   for port in teensy_serial_ports() {
      if !matches_filter(port.serial_number.as_deref(), &port.port_name, options) {
         continue;
      }
      found = true;

      let mut serial = serialport::new(&port.port_name, 115_200)
         .timeout(Duration::from_millis(100))
         .open()
         .with_context(|| format!("failed to open {}", port.port_name))?;
      serial.set_baud_rate(134)?;
      let _ = serial.set_baud_rate(115_200);
      rebooted = true;
   }

   if !found {
      bail!("no Teensy device found to reboot into bootloader");
   }
   if !rebooted {
      bail!("found Teensy device but could not reboot it into bootloader");
   }

   Ok(())
}

fn matches_filter(serial: Option<&str>, path: &str, options: &UploadOptions) -> bool {
   options
      .board_filter
      .as_ref()
      .is_none_or(|filter| serial.is_some_and(|serial| serial == filter) || path.contains(filter))
}

struct HalfKaySettings {
   version:     u8,
   min_address: u32,
   max_address: u32,
   block_size:  usize,
}

impl HalfKaySettings {
   fn for_model(model: Model) -> Result<Self> {
      let settings = match model {
         Model::TeensyPp10 => {
            Self {
               version:     1,
               min_address: 0,
               max_address: 0xFC00,
               block_size:  256,
            }
         },
         Model::Teensy20 => {
            Self {
               version:     1,
               min_address: 0,
               max_address: 0x7E00,
               block_size:  128,
            }
         },
         Model::TeensyPp20 => {
            Self {
               version:     2,
               min_address: 0,
               max_address: 0x1FC00,
               block_size:  256,
            }
         },
         Model::Teensy30 => {
            Self {
               version:     3,
               min_address: 0,
               max_address: 0x20000,
               block_size:  1024,
            }
         },
         Model::Teensy31 | Model::Teensy32 => {
            Self {
               version:     3,
               min_address: 0,
               max_address: 0x40000,
               block_size:  1024,
            }
         },
         Model::Teensy35 => {
            Self {
               version:     3,
               min_address: 0,
               max_address: 0x80000,
               block_size:  1024,
            }
         },
         Model::Teensy36 => {
            Self {
               version:     3,
               min_address: 0,
               max_address: 0x100000,
               block_size:  1024,
            }
         },
         Model::TeensyLc => {
            Self {
               version:     3,
               min_address: 0,
               max_address: 0xF800,
               block_size:  512,
            }
         },
         Model::Teensy40Beta1 | Model::Teensy40 => {
            Self {
               version:     3,
               min_address: 0x6000_0000,
               max_address: 0x601F_0000,
               block_size:  1024,
            }
         },
         Model::Teensy41 => {
            Self {
               version:     3,
               min_address: 0x6000_0000,
               max_address: 0x607C_0000,
               block_size:  1024,
            }
         },
         Model::TeensyMicroMod => {
            Self {
               version:     3,
               min_address: 0x6000_0000,
               max_address: 0x60FC_0000,
               block_size:  1024,
            }
         },
         Model::Generic | Model::Teensy => bail!("unsupported Teensy model: {}", model.info().name),
      };

      Ok(settings)
   }
}

pub fn reboot_to_bootloader(board_filter: Option<&str>) -> Result<()> {
   let api = HidApi::new()?;
   let filter = board_filter.map(|s| s.to_string());
   let options = UploadOptions {
      board_filter: filter,
      ..UploadOptions::default()
   };

   let mut found_seremu = false;
   for info in api.device_list().filter(|info| is_teensy_seremu(info)) {
      if !matches_filter(
         info.serial_number(),
         &info.path().to_string_lossy(),
         &options,
      ) {
         continue;
      }
      found_seremu = true;
      let device = info.open_device(&api)?;
      device.send_feature_report(&[0, 0xA9, 0x45, 0xC2, 0x6B])?;
   }

   for port in teensy_serial_ports() {
      if !matches_filter(port.serial_number.as_deref(), &port.port_name, &options) {
         continue;
      }

      let mut serial = serialport::new(&port.port_name, 115_200)
         .timeout(Duration::from_millis(100))
         .open()
         .with_context(|| format!("failed to open {}", port.port_name))?;
      serial.set_baud_rate(134)?;
      let _ = serial.set_baud_rate(115_200);
      return Ok(());
   }

   if !found_seremu {
      bail!("no Teensy device found to reboot");
   }

   Ok(())
}

pub fn reset_device(board_filter: Option<&str>) -> Result<()> {
   let api = HidApi::new()?;
   let filter = board_filter.map(|s| s.to_string());
   let options = UploadOptions {
      board_filter: filter,
      ..UploadOptions::default()
   };

   if let Some(bootloader) = open_bootloader_once(&api, &options)? {
      let (device, model) = bootloader;
      let settings = HalfKaySettings::for_model(model)?;
      halfkay_send(
         &device,
         settings.version,
         settings.block_size,
         0x00FF_FFFF,
         &[],
         25,
         Duration::from_millis(20),
      )?;
      return Ok(());
   }

   reboot_running_teensy(&api, &options)?;

   loop {
      let api = HidApi::new()?;
      if let Some(bootloader) = open_bootloader_once(&api, &options)? {
         let (device, model) = bootloader;
         let settings = HalfKaySettings::for_model(model)?;
         halfkay_send(
            &device,
            settings.version,
            settings.block_size,
            0x00FF_FFFF,
            &[],
            25,
            Duration::from_millis(20),
         )?;
         return Ok(());
      }
      thread::sleep(Duration::from_millis(100));
   }
}

fn halfkay_send(
   device: &HidDevice,
   version: u8,
   block_size: usize,
   address: u32,
   data: &[u8],
   tries: usize,
   delay: Duration,
) -> Result<()> {
   let mut report = vec![0; block_size + 65];
   let size = match version {
      1 => {
         report[1] = address as u8;
         report[2] = (address >> 8) as u8;
         report[3..3 + data.len()].copy_from_slice(data);
         block_size + 3
      },
      2 => {
         report[1] = (address >> 8) as u8;
         report[2] = (address >> 16) as u8;
         report[3..3 + data.len()].copy_from_slice(data);
         block_size + 3
      },
      3 => {
         report[1] = address as u8;
         report[2] = (address >> 8) as u8;
         report[3] = (address >> 16) as u8;
         report[65..65 + data.len()].copy_from_slice(data);
         block_size + 65
      },
      _ => bail!("unsupported HalfKay protocol version {version}"),
   };

   let mut last_error = None;
   for _ in 0..tries {
      match device.write(&report[..size]) {
         Ok(_) => return Ok(()),
         Err(error) => {
            last_error = Some(error);
            thread::sleep(delay);
         },
      }
   }

   Err(last_error.context("failed to write HalfKay report")?.into())
}
