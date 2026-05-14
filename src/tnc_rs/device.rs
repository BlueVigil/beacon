// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::BTreeMap;

use anyhow::Result;
use hidapi::HidApi;
use serialport::SerialPortType;

use super::model::Model;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Device {
   pub raw_line:     String,
   pub label:        String,
   pub tag:          String,
   pub model:        Model,
   pub location:     String,
   pub serial:       Option<String>,
   pub description:  Option<String>,
   pub capabilities: Vec<Capability>,
   pub interfaces:   Vec<Interface>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Interface {
   pub name: String,
   pub path: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Capability {
   Unique,
   Void,
   Run,
   Upload,
   Encrypt,
   Lock,
   Locked,
   Reset,
   Rtc,
   Reboot,
   Serial,
}

pub fn list_devices() -> Result<Vec<Device>> {
   let api = HidApi::new()?;
   let mut seen: BTreeMap<String, Device> = BTreeMap::new();

   for info in api
      .device_list()
      .filter(|info| is_teensy_hid(info) || is_teensy_seremu(info))
   {
      let is_bootloader = is_teensy_hid(info);
      let model = model_from_halfkay_usage(info.usage()).unwrap_or(Model::Teensy);
      let serial = info.serial_number().map(ToString::to_string);
      let location = info.path().to_string_lossy().into_owned();
      let interface_name = if is_bootloader { "HalfKay" } else { "Seremu" };
      let raw_tag = serial.clone().unwrap_or_else(|| location.clone());
      let tag = if serial.is_some() {
         format!("{raw_tag}-Teensy")
      } else {
         raw_tag.clone()
      };
      let key = dedup_key(&raw_tag, is_bootloader);
      let description = info.product_string().map(ToString::to_string);

      match seen.get_mut(&key) {
         Some(existing) => {
            existing.interfaces.push(Interface {
               name: interface_name.to_string(),
               path: location,
            });
         },
         None => {
            let label = if is_bootloader {
               format!("add {tag} {} (HalfKay)", model.info().name)
            } else {
               format!(
                  "add {tag} {}",
                  description
                     .clone()
                     .unwrap_or_else(|| model.info().name.to_string())
               )
            };
            let iface_path = location.clone();
            seen.insert(key, Device {
               raw_line: label.clone(),
               label,
               tag,
               model,
               location,
               serial,
               description,
               capabilities: if is_bootloader {
                  vec![Capability::Upload, Capability::Reset]
               } else {
                  vec![Capability::Run, Capability::Serial, Capability::Reboot]
               },
               interfaces: vec![Interface {
                  name: interface_name.to_string(),
                  path: iface_path,
               }],
            });
         },
      }
   }

   for (key, serial_dev) in list_serial_devices()? {
      match seen.get_mut(&key) {
         Some(existing) => {
            existing.label = serial_dev.label.clone();
            existing.raw_line = serial_dev.raw_line.clone();
            existing.description = serial_dev.description.clone();
            existing.interfaces.extend(serial_dev.interfaces);
         },
         None => {
            seen.insert(key, serial_dev);
         },
      }
   }

   Ok(seen.into_values().collect())
}

fn list_serial_devices() -> Result<BTreeMap<String, Device>> {
   let mut seen = BTreeMap::new();

   for port in serialport::available_ports().unwrap_or_default() {
      let SerialPortType::UsbPort(info) = &port.port_type else {
         continue;
      };
      if !is_teensy_vid_pid(info.vid, info.pid) {
         continue;
      }

      let serial = info.serial_number.clone();
      let tag = serial.clone().unwrap_or_else(|| port.port_name.clone());
      let model = model_from_product_string(info.product.as_deref()).unwrap_or(Model::Teensy);
      let key = dedup_key(&tag, false);
      let label = format!("add {tag} {} (USB Serial)", model.info().name);

      seen.insert(key, Device {
         raw_line: label.clone(),
         label,
         tag,
         model,
         location: port.port_name.clone(),
         serial,
         description: info.product.clone(),
         capabilities: vec![Capability::Run, Capability::Serial, Capability::Reboot],
         interfaces: vec![Interface {
            name: "Serial".to_string(),
            path: port.port_name,
         }],
      });
   }

   Ok(seen)
}

fn dedup_key(tag: &str, is_bootloader: bool) -> String {
   format!("{}:{}", if is_bootloader { "boot" } else { "run" }, tag)
}

pub(crate) fn is_teensy_hid(info: &hidapi::DeviceInfo) -> bool {
   info.vendor_id() == 0x16C0 && info.usage_page() == 0xFF9C
}

pub(crate) fn is_teensy_seremu(info: &hidapi::DeviceInfo) -> bool {
   info.vendor_id() == 0x16C0 && info.usage_page() == 0xFFC9
}

pub(crate) fn is_teensy_vid_pid(vid: u16, pid: u16) -> bool {
   vid == 0x16C0
      && matches!(
         pid,
         0x0476
            | 0x0478
            | 0x0482
            | 0x0483
            | 0x0484
            | 0x0485
            | 0x0486
            | 0x0487
            | 0x0488
            | 0x0489
            | 0x048A
            | 0x048B
            | 0x048C
            | 0x04D0
            | 0x04D1
            | 0x04D2
            | 0x04D3
            | 0x04D4
            | 0x04D5
            | 0x04D9
      )
}

pub(crate) fn model_from_halfkay_usage(usage: u16) -> Option<Model> {
   Some(match usage {
      0x1A => Model::TeensyPp10,
      0x1B => Model::Teensy20,
      0x1C => Model::TeensyPp20,
      0x1D => Model::Teensy30,
      0x1E => Model::Teensy31,
      0x20 => Model::TeensyLc,
      0x21 => Model::Teensy32,
      0x1F => Model::Teensy35,
      0x22 => Model::Teensy36,
      0x23 => Model::Teensy40Beta1,
      0x24 => Model::Teensy40,
      0x25 => Model::Teensy41,
      0x26 => Model::TeensyMicroMod,
      _ => return None,
   })
}

pub(crate) fn model_from_product_string(product: Option<&str>) -> Option<Model> {
   let product = product?;
   if product.contains("Teensy 4.1") {
      Some(Model::Teensy41)
   } else if product.contains("Teensy 4.0") {
      Some(Model::Teensy40)
   } else if product.contains("Teensy 3.6") {
      Some(Model::Teensy36)
   } else if product.contains("Teensy 3.5") {
      Some(Model::Teensy35)
   } else if product.contains("Teensy 3.2") {
      Some(Model::Teensy32)
   } else if product.contains("Teensy 3.1") {
      Some(Model::Teensy31)
   } else if product.contains("Teensy 3.0") {
      Some(Model::Teensy30)
   } else if product.contains("Teensy LC") {
      Some(Model::TeensyLc)
   } else if product.contains("Teensy MicroMod") {
      Some(Model::TeensyMicroMod)
   } else if product.contains("Teensy++ 2.0") {
      Some(Model::TeensyPp20)
   } else if product.contains("Teensy++ 1.0") {
      Some(Model::TeensyPp10)
   } else if product.contains("Teensy 2.0") {
      Some(Model::Teensy20)
   } else {
      None
   }
}
