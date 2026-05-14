// SPDX-License-Identifier: AGPL-3.0-only

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Model {
   Generic,
   Teensy,
   TeensyPp10,
   Teensy20,
   TeensyPp20,
   Teensy30,
   Teensy31,
   TeensyLc,
   Teensy32,
   Teensy35,
   Teensy36,
   Teensy40Beta1,
   Teensy40,
   Teensy41,
   TeensyMicroMod,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelInfo {
   pub model:    Model,
   pub priority: u8,
   pub name:     &'static str,
   pub mcu:      Option<&'static str>,
}

impl Model {
   pub fn info(self) -> ModelInfo {
      match self {
         Model::Generic => {
            ModelInfo {
               model:    self,
               priority: 0,
               name:     "Generic",
               mcu:      None,
            }
         },
         Model::Teensy => {
            ModelInfo {
               model:    self,
               priority: 1,
               name:     "Teensy",
               mcu:      None,
            }
         },
         Model::TeensyPp10 => {
            ModelInfo {
               model:    self,
               priority: 1,
               name:     "Teensy++ 1.0",
               mcu:      Some("at90usb646"),
            }
         },
         Model::Teensy20 => {
            ModelInfo {
               model:    self,
               priority: 1,
               name:     "Teensy 2.0",
               mcu:      Some("atmega32u4"),
            }
         },
         Model::TeensyPp20 => {
            ModelInfo {
               model:    self,
               priority: 1,
               name:     "Teensy++ 2.0",
               mcu:      Some("at90usb1286"),
            }
         },
         Model::Teensy30 => {
            ModelInfo {
               model:    self,
               priority: 1,
               name:     "Teensy 3.0",
               mcu:      Some("mk20dx128"),
            }
         },
         Model::Teensy31 => {
            ModelInfo {
               model:    self,
               priority: 1,
               name:     "Teensy 3.1",
               mcu:      Some("mk20dx256"),
            }
         },
         Model::TeensyLc => {
            ModelInfo {
               model:    self,
               priority: 1,
               name:     "Teensy LC",
               mcu:      Some("mkl26z64"),
            }
         },
         Model::Teensy32 => {
            ModelInfo {
               model:    self,
               priority: 1,
               name:     "Teensy 3.2",
               mcu:      Some("mk20dx256"),
            }
         },
         Model::Teensy35 => {
            ModelInfo {
               model:    self,
               priority: 1,
               name:     "Teensy 3.5",
               mcu:      Some("mk64fx512"),
            }
         },
         Model::Teensy36 => {
            ModelInfo {
               model:    self,
               priority: 1,
               name:     "Teensy 3.6",
               mcu:      Some("mk66fx1m0"),
            }
         },
         Model::Teensy40Beta1 => {
            ModelInfo {
               model:    self,
               priority: 1,
               name:     "Teensy 4.0 (beta 1)",
               mcu:      Some("imxrt_b1"),
            }
         },
         Model::Teensy40 => {
            ModelInfo {
               model:    self,
               priority: 1,
               name:     "Teensy 4.0",
               mcu:      Some("imxrt"),
            }
         },
         Model::Teensy41 => {
            ModelInfo {
               model:    self,
               priority: 1,
               name:     "Teensy 4.1",
               mcu:      Some("imxrt_t41"),
            }
         },
         Model::TeensyMicroMod => {
            ModelInfo {
               model:    self,
               priority: 1,
               name:     "Teensy MicroMod",
               mcu:      Some("imxrt_mm"),
            }
         },
      }
   }
}
