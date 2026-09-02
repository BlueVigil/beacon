// SPDX-License-Identifier: AGPL-3.0-only

use std::{
   collections::BTreeMap,
   path::Path,
};

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
   let mut seen: BTreeMap<String, Device> = BTreeMap::new();

   if let Ok(api) = HidApi::new() {
      collect_hid_devices(&api, &mut seen);
   }

   for (key, serial_dev) in list_serial_devices() {
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

fn collect_hid_devices(api: &HidApi, seen: &mut BTreeMap<String, Device>) {
   for info in api.device_list().filter(|info| is_listed_hid(info)) {
      let is_bootloader = is_teensy_hid(info);
      let model = identify_hid_model(info);
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
}

#[derive(Clone, Debug)]
pub(crate) struct TeensySerialPort {
   pub port_name:     String,
   pub serial_number: Option<String>,
   pub product:       Option<String>,
}

pub(crate) fn teensy_serial_ports() -> Vec<TeensySerialPort> {
   serialport::available_ports()
      .unwrap_or_default()
      .into_iter()
      .filter_map(|port| {
         let identity = usb_identity(&port)?;
         if !is_teensy_usb(identity.vid, identity.pid, identity.product.as_deref()) {
            return None;
         }
         Some(TeensySerialPort {
            port_name:     port.port_name,
            serial_number: identity.serial_number,
            product:       identity.product,
         })
      })
      .collect()
}

struct UsbIdentity {
   vid:           u16,
   pid:           u16,
   serial_number: Option<String>,
   product:       Option<String>,
}

fn usb_identity(port: &serialport::SerialPortInfo) -> Option<UsbIdentity> {
   if let SerialPortType::UsbPort(info) = &port.port_type {
      return Some(UsbIdentity {
         vid:           info.vid,
         pid:           info.pid,
         serial_number: info.serial_number.clone(),
         product:       info.product.clone(),
      });
   }

   linux_sysfs_usb(&port.port_name)
}

fn linux_sysfs_usb(port_name: &str) -> Option<UsbIdentity> {
   if !cfg!(target_os = "linux") {
      return None;
   }
   let name = Path::new(port_name).file_name()?.to_str()?;
   let (vid, pid, serial_number, product) =
      usb_ids_from_sysfs_dir(&Path::new("/sys/class/tty").join(name).join("device"))?;
   Some(UsbIdentity {
      vid,
      pid,
      serial_number,
      product,
   })
}

fn usb_ids_from_sysfs_dir(start: &Path) -> Option<(u16, u16, Option<String>, Option<String>)> {
   let mut dir = std::fs::canonicalize(start).ok()?;
   for _ in 0..12 {
      if let (Ok(vendor), Ok(product_id)) = (
         std::fs::read_to_string(dir.join("idVendor")),
         std::fs::read_to_string(dir.join("idProduct")),
      ) {
         let vid = parse_hex_u16(&vendor)?;
         let pid = parse_hex_u16(&product_id)?;
         let serial_number = read_sysfs_string(&dir.join("serial"));
         let product = read_sysfs_string(&dir.join("product"));
         return Some((vid, pid, serial_number, product));
      }
      if !dir.pop() {
         break;
      }
   }
   None
}

fn parse_hex_u16(raw: &str) -> Option<u16> {
   u16::from_str_radix(raw.trim().trim_start_matches("0x"), 16).ok()
}

fn read_sysfs_string(path: &Path) -> Option<String> {
   let value = std::fs::read_to_string(path).ok()?;
   let value = value.trim();
   if value.is_empty() {
      None
   } else {
      Some(value.to_string())
   }
}

fn list_serial_devices() -> BTreeMap<String, Device> {
   let mut seen = BTreeMap::new();

   for port in teensy_serial_ports() {
      let serial = port.serial_number.clone();
      let tag = serial.clone().unwrap_or_else(|| port.port_name.clone());
      let model = model_from_product_string(port.product.as_deref()).unwrap_or(Model::Teensy);
      let key = dedup_key(&tag, false);
      let label = format!("add {tag} {} (USB Serial)", model.info().name);

      seen.insert(key, Device {
         raw_line: label.clone(),
         label,
         tag,
         model,
         location: port.port_name.clone(),
         serial,
         description: port.product.clone(),
         capabilities: vec![Capability::Run, Capability::Serial, Capability::Reboot],
         interfaces: vec![Interface {
            name: "Serial".to_string(),
            path: port.port_name,
         }],
      });
   }

   seen
}

fn dedup_key(tag: &str, is_bootloader: bool) -> String {
   format!("{}:{}", if is_bootloader { "boot" } else { "run" }, tag)
}

fn is_listed_hid(info: &hidapi::DeviceInfo) -> bool {
   is_teensy_hid(info)
      || is_teensy_seremu(info)
      || (info.vendor_id() == 0x16C0 && matches!(info.usage_page(), 0xFFAB | 0xFF00))
}

pub(crate) fn is_teensy_hid(info: &hidapi::DeviceInfo) -> bool {
   hid_is_bootloader(info.vendor_id(), info.product_id(), info.usage_page())
}

pub(crate) fn is_teensy_seremu(info: &hidapi::DeviceInfo) -> bool {
   hid_is_seremu(info.vendor_id(), info.product_id(), info.usage_page())
}

fn hid_is_bootloader(vid: u16, pid: u16, usage_page: u16) -> bool {
   if vid != 0x16C0 {
      return false;
   }
   usage_page == 0xFF9C || (usage_page == 0 && is_halfkay_pid(pid))
}

fn hid_is_seremu(vid: u16, pid: u16, usage_page: u16) -> bool {
   if vid != 0x16C0 && vid != 0x1FC9 {
      return false;
   }
   if usage_page == 0xFFC9 {
      return true;
   }
   usage_page == 0 && is_teensy_vid_pid(vid, pid) && !is_halfkay_pid(pid)
}

fn is_halfkay_pid(pid: u16) -> bool {
   matches!(pid, 0x0476 | 0x0478)
}

fn is_teensy_usb(vid: u16, pid: u16, product: Option<&str>) -> bool {
   is_teensy_vid_pid(vid, pid) || product.is_some_and(|product| product.contains("Teensy"))
}

pub(crate) fn is_teensy_vid_pid(vid: u16, pid: u16) -> bool {
   matches!(
      (vid, pid),
      (
         0x16C0,
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
      ) | (0x1FC9, 0x0135)
   )
}

pub(crate) fn identify_hid_model(info: &hidapi::DeviceInfo) -> Model {
   model_from_halfkay_usage(info.usage())
      .or_else(|| model_from_bcd_device(info.release_number()))
      .or_else(|| model_from_product_string(info.product_string()))
      .unwrap_or(Model::Teensy)
}

fn model_from_bcd_device(bcd_device: u16) -> Option<Model> {
   Some(match bcd_device {
      0x274 => Model::Teensy30,
      0x275 => Model::Teensy31,
      0x273 => Model::TeensyLc,
      0x276 => Model::Teensy35,
      0x277 => Model::Teensy36,
      0x278 => Model::Teensy40Beta1,
      0x279 => Model::Teensy40,
      0x280 => Model::Teensy41,
      0x281 => Model::TeensyMicroMod,
      _ => return None,
   })
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

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn teensy_usb_serial_pid_matches() {
      assert!(is_teensy_vid_pid(0x16C0, 0x0483));
      assert!(is_teensy_vid_pid(0x1FC9, 0x0135));
      assert!(!is_teensy_vid_pid(0x0403, 0x6001));
   }

   #[test]
   fn linux_usage_page_zero_still_matches_hid() {
      assert!(hid_is_bootloader(0x16C0, 0x0478, 0));
      assert!(hid_is_bootloader(0x16C0, 0x0478, 0xFF9C));
      assert!(!hid_is_bootloader(0x16C0, 0x0483, 0));
      assert!(hid_is_seremu(0x16C0, 0x0483, 0));
      assert!(hid_is_seremu(0x16C0, 0x0486, 0xFFC9));
      assert!(!hid_is_seremu(0x0403, 0x6001, 0));
   }

   #[test]
   fn product_string_identifies_teensy_without_vid() {
      assert!(is_teensy_usb(0x0403, 0x6001, Some("Teensy USB Serial")));
      assert!(!is_teensy_usb(0x0403, 0x6001, Some("FT232R USB UART")));
   }

   #[test]
   fn sysfs_walk_reads_usb_ids() {
      let root = std::env::temp_dir().join(format!("beacon-sysfs-{}", std::process::id()));
      let usb = root.join("usb");
      let start = usb.join("iface").join("tty").join("device");
      std::fs::create_dir_all(&start).unwrap();
      std::fs::write(usb.join("idVendor"), "16c0\n").unwrap();
      std::fs::write(usb.join("idProduct"), "0483\n").unwrap();
      std::fs::write(usb.join("product"), "Teensy USB Serial\n").unwrap();
      std::fs::write(usb.join("serial"), "12345\n").unwrap();

      let (vid, pid, serial, product) = usb_ids_from_sysfs_dir(&start).unwrap();
      let _ = std::fs::remove_dir_all(&root);

      assert_eq!(vid, 0x16C0);
      assert_eq!(pid, 0x0483);
      assert_eq!(serial.as_deref(), Some("12345"));
      assert_eq!(product.as_deref(), Some("Teensy USB Serial"));
   }

   #[test]
   fn bcd_device_identifies_teensy_41() {
      assert_eq!(model_from_bcd_device(0x280), Some(Model::Teensy41));
      assert_eq!(model_from_bcd_device(0), None);
   }
}
