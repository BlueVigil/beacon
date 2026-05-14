// SPDX-License-Identifier: AGPL-3.0-only

use std::{
   fs,
   path::Path,
};

use anyhow::{
   Context as _,
   Result,
   bail,
};

use super::model::Model;

const MAX_PROGRAMS: usize = 4;
const MAX_SEGMENTS: usize = 16;
const MAX_SIZE: usize = 32 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FirmwareFormat {
   IntelHex,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FirmwareSegment {
   pub address: u32,
   pub data:    Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FirmwareProgram {
   pub segments:    Vec<FirmwareSegment>,
   pub min_address: usize,
   pub max_address: usize,
   pub total_size:  usize,
}

impl Default for FirmwareProgram {
   fn default() -> Self {
      Self {
         segments:    Vec::new(),
         min_address: usize::MAX,
         max_address: 0,
         total_size:  0,
      }
   }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Firmware {
   pub name:     String,
   pub filename: String,
   pub format:   FirmwareFormat,
   pub programs: Vec<FirmwareProgram>,
}

pub fn load_firmware_file(path: &Path) -> Result<Firmware> {
   let format = format_from_path(path)?;
   let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
   let filename = path.display().to_string();
   let name = path
      .file_name()
      .and_then(|name| name.to_str())
      .unwrap_or(&filename)
      .to_string();

   match format {
      FirmwareFormat::IntelHex => parse_ihex(filename, name, &bytes),
   }
}

pub fn identify_firmware_file(path: &Path) -> Result<Vec<Model>> {
   Ok(load_firmware_file(path)?.identify_models())
}

pub fn is_hex_file(path: &Path) -> bool {
   path
      .extension()
      .and_then(|extension| extension.to_str())
      .is_some_and(|extension| extension.eq_ignore_ascii_case("hex"))
}

impl Firmware {
   pub fn identify_models(&self) -> Vec<Model> {
      let Some(program) = self.programs.first() else {
         return Vec::new();
      };

      identify_teensy_models(program)
   }
}

impl FirmwareProgram {
   pub fn extract(&self, address: u32, buf: &mut [u8]) -> usize {
      let mut total_len = 0;

      for segment in &self.segments {
         let segment_start = segment.address;
         let segment_end = segment.address + segment.data.len() as u32;
         let request_end = address + buf.len() as u32;

         if address >= segment_start && address < segment_end {
            let delta = (address - segment_start) as usize;
            let len = (segment.data.len() - delta).min(buf.len());
            buf[..len].copy_from_slice(&segment.data[delta..delta + len]);
            total_len += len;
         } else if address < segment_start && request_end > segment_start {
            let delta = (segment_start - address) as usize;
            let len = segment.data.len().min(buf.len() - delta);
            buf[delta..delta + len].copy_from_slice(&segment.data[..len]);
            total_len += len;
         }
      }

      total_len
   }

   fn add_data(&mut self, address: u32, data: &[u8]) -> Result<()> {
      if data.is_empty() {
         return Ok(());
      }

      if let Some(segment) = self
         .segments
         .last_mut()
         .filter(|segment| address as usize + data.len() <= segment.address as usize + 1_048_576)
      {
         let offset = address
            .checked_sub(segment.address)
            .context("IHEX addresses moved backwards within a segment")?
            as usize;
         let new_len = offset + data.len();
         if new_len > segment.data.len() {
            let total_size = self.total_size - segment.data.len() + new_len;
            if total_size > MAX_SIZE {
               bail!("firmware has excessive size");
            }
            segment.data.resize(new_len, 0);
            self.total_size = total_size;
         }
         segment.data[offset..offset + data.len()].copy_from_slice(data);
         return Ok(());
      }

      if self.segments.len() >= MAX_SEGMENTS {
         bail!("firmware has too many segments");
      }
      if self.total_size + data.len() > MAX_SIZE {
         bail!("firmware has excessive size");
      }

      self.segments.push(FirmwareSegment {
         address,
         data: data.to_vec(),
      });
      self.total_size += data.len();
      Ok(())
   }

   fn finish_ranges(&mut self) {
      self.min_address = usize::MAX;
      self.max_address = 0;

      for segment in &self.segments {
         self.min_address = self.min_address.min(segment.address as usize);
         self.max_address = self
            .max_address
            .max(segment.address as usize + segment.data.len());
      }
   }

   fn find_segment(&self, address: u32) -> Option<&FirmwareSegment> {
      self.segments.iter().rev().find(|segment| {
         let start = segment.address;
         let end = segment.address + segment.data.len() as u32;
         address >= start && address < end
      })
   }
}

fn format_from_path(path: &Path) -> Result<FirmwareFormat> {
   match path.extension().and_then(|extension| extension.to_str()) {
      Some(extension) if extension.eq_ignore_ascii_case("hex") => Ok(FirmwareFormat::IntelHex),
      Some(extension) => bail!("unsupported firmware extension: .{extension}"),
      None => bail!("firmware has no file extension: {}", path.display()),
   }
}

fn parse_ihex(filename: String, name: String, bytes: &[u8]) -> Result<Firmware> {
   let text = std::str::from_utf8(bytes).context("IHEX firmware is not valid UTF-8")?;
   let mut program = FirmwareProgram::default();
   let mut offset1 = 0u32;
   let mut offset2 = 0u32;
   let mut saw_eof = false;

   for (index, raw_line) in text.lines().enumerate() {
      let line = raw_line.trim_end_matches('\r');
      if line.is_empty() {
         continue;
      }

      let record = parse_ihex_line(line)
         .with_context(|| format!("IHEX parse error on line {} in '{filename}'", index + 1))?;

      match record.kind {
         0x00 => {
            let address = record.address as u32 + offset1 + offset2;
            program.add_data(address, &record.data)?;
         },
         0x01 => {
            if !record.data.is_empty() {
               bail!("IHEX EOF record contains data");
            }
            saw_eof = true;
            break;
         },
         0x02 => {
            if record.data.len() != 2 {
               bail!("invalid IHEX extended segment address record");
            }
            offset2 = (u16::from_be_bytes([record.data[0], record.data[1]]) as u32) << 4;
         },
         0x03 | 0x05 => {
            if record.data.len() != 4 {
               bail!("invalid IHEX start address record");
            }
         },
         0x04 => {
            if record.data.len() != 2 {
               bail!("invalid IHEX extended linear address record");
            }
            offset1 = (u16::from_be_bytes([record.data[0], record.data[1]]) as u32) << 16;
         },
         kind => bail!("unsupported IHEX record type {kind}"),
      }
   }

   if !saw_eof {
      bail!("missing EOF record in '{filename}'");
   }

   program.finish_ranges();

   Ok(Firmware {
      name,
      filename,
      format: FirmwareFormat::IntelHex,
      programs: vec![program],
   })
}

#[derive(Debug, Eq, PartialEq)]
struct IhexRecord {
   address: u16,
   kind:    u8,
   data:    Vec<u8>,
}

fn parse_ihex_line(line: &str) -> Result<IhexRecord> {
   let bytes = line.as_bytes();
   if bytes.first() != Some(&b':') {
      bail!("missing record marker");
   }

   let byte_count = parse_hex_byte(bytes, 1)? as usize;
   let expected_len = 11 + 2 * byte_count;
   if bytes.len() != expected_len {
      bail!("invalid record length");
   }

   let mut sum = 0u8;
   let mut fields = Vec::with_capacity(4 + byte_count + 1);
   for offset in (1..bytes.len()).step_by(2) {
      let value = parse_hex_byte(bytes, offset)?;
      sum = sum.wrapping_add(value);
      fields.push(value);
   }

   if sum != 0 {
      bail!("invalid record checksum");
   }

   let address = u16::from_be_bytes([fields[1], fields[2]]);
   let kind = fields[3];
   let data = fields[4..4 + byte_count].to_vec();

   Ok(IhexRecord {
      address,
      kind,
      data,
   })
}

fn parse_hex_byte(bytes: &[u8], offset: usize) -> Result<u8> {
   let hi = parse_hex_nibble(bytes.get(offset).copied())?;
   let lo = parse_hex_nibble(bytes.get(offset + 1).copied())?;
   Ok((hi << 4) | lo)
}

fn parse_hex_nibble(byte: Option<u8>) -> Result<u8> {
   match byte {
      Some(b'0'..=b'9') => Ok(byte.unwrap() - b'0'),
      Some(b'a'..=b'f') => Ok(byte.unwrap() - b'a' + 10),
      Some(b'A'..=b'F') => Ok(byte.unwrap() - b'A' + 10),
      _ => bail!("invalid hex digit"),
   }
}

fn identify_teensy_models(program: &FirmwareProgram) -> Vec<Model> {
   if let Some(segment) = program.find_segment(0x6000_0000)
      && segment.data.len() >= 8
      && read_u64_le(&segment.data[0..8]) == 0x5601_0000_4246_4346
   {
      if segment.data.len() > 84 {
         match read_u32_le(&segment.data[80..84]) {
            0x0080_0000 => return vec![Model::Teensy41],
            0x0100_0000 => return vec![Model::TeensyMicroMod],
            _ => {},
         }
      }

      return vec![Model::Teensy40, Model::Teensy40Beta1];
   }

   if let Some(segment) = program.find_segment(0) {
      const TEENSY3_STARTUP_SIZE: usize = 0x400;
      if segment.data.len() >= TEENSY3_STARTUP_SIZE {
         let stack_addr = read_u32_le(&segment.data[0..4]);
         let mut end_vector_addr = read_u32_le(&segment.data[4..8]) & !1;

         if end_vector_addr >= TEENSY3_STARTUP_SIZE as u32 {
            for i in (0..TEENSY3_STARTUP_SIZE - 8).step_by(4) {
               if read_u64_le(&segment.data[i..i + 8]) == 0xFFFF_FFFF_FFFF_FFFF {
                  end_vector_addr = i as u32;
                  break;
               }
            }
         }

         match ((stack_addr as u64) << 32) | end_vector_addr as u64 {
            0x2000_2000_0000_00F8 => return vec![Model::Teensy30],
            0x2000_8000_0000_01BC => return vec![Model::Teensy31, Model::Teensy32],
            0x2000_1800_0000_00C0 => return vec![Model::TeensyLc],
            0x2002_0000_0000_0198 | 0x2002_FFFC_0000_0198 | 0x2002_FFF8_0000_0198 => {
               return vec![Model::Teensy35];
            },
            0x2003_0000_0000_01D0 => return vec![Model::Teensy36],
            _ => {},
         }
      }
   }

   if program.max_address <= 130_048 {
      for segment in &program.segments {
         if segment.data.len() < 8 {
            continue;
         }

         for window in segment.data.windows(8) {
            match read_u64_le(window) {
               0x94F8_CFFF_7E00_940C => return vec![Model::TeensyPp10],
               0x94F8_CFFF_3F00_940C => return vec![Model::Teensy20],
               0x94F8_CFFF_FE00_940C => return vec![Model::TeensyPp20],
               _ => {},
            }
         }
      }
   }

   Vec::new()
}

fn read_u32_le(bytes: &[u8]) -> u32 {
   u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn read_u64_le(bytes: &[u8]) -> u64 {
   u64::from_le_bytes([
      bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
   ])
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn parses_ihex_record() {
      let record = parse_ihex_line(":10010000214601360121470136007EFE09D2190140").unwrap();

      assert_eq!(record.address, 0x0100);
      assert_eq!(record.kind, 0);
      assert_eq!(record.data.len(), 16);
   }

   #[test]
   fn rejects_bad_checksum() {
      assert!(parse_ihex_line(":00000001FE").is_err());
   }

   #[test]
   fn identifies_teensy_41_flash_config() {
      let mut data = vec![0; 88];
      data[0..8].copy_from_slice(&0x5601_0000_4246_4346u64.to_le_bytes());
      data[80..84].copy_from_slice(&0x0080_0000u32.to_le_bytes());
      let program = FirmwareProgram {
         segments:    vec![FirmwareSegment {
            address: 0x6000_0000,
            data,
         }],
         min_address: 0x6000_0000,
         max_address: 0x6000_0000 + 88,
         total_size:  88,
      };

      assert_eq!(identify_teensy_models(&program), vec![Model::Teensy41]);
   }
}
