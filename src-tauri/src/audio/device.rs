use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Device, Host};
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub is_default: bool,
    pub default_sample_rate: u32,
    pub channels: u16,
}

pub struct AudioDeviceManager {
    host: Host,
}

impl AudioDeviceManager {
    pub fn new() -> Self {
        Self {
            host: cpal::default_host(),
        }
    }

    pub fn list_output_devices(&self) -> Vec<AudioDeviceInfo> {
        let default_device_name = self.host.default_output_device().and_then(|d| d.name().ok());
        let mut list = Vec::new();

        if let Ok(devices) = self.host.output_devices() {
            for device in devices {
                if let Ok(name) = device.name() {
                    let is_default = default_device_name.as_ref().map(|n| n == &name).unwrap_or(false);
                    let (sample_rate, channels) = device
                        .default_output_config()
                        .map(|c| (c.sample_rate().0, c.channels()))
                        .unwrap_or((44100, 2));

                    list.push(AudioDeviceInfo {
                        name,
                        is_default,
                        default_sample_rate: sample_rate,
                        channels,
                    });
                }
            }
        }
        list
    }

    pub fn get_output_device(&self, name_opt: Option<&str>) -> Option<Device> {
        match name_opt {
            Some(name) => {
                if let Ok(devices) = self.host.output_devices() {
                    for dev in devices {
                        if let Ok(n) = dev.name() {
                            if n == name {
                                return Some(dev);
                            }
                        }
                    }
                }
                warn!("Requested audio device '{}' not found, falling back to default", name);
                self.host.default_output_device()
            }
            None => self.host.default_output_device(),
        }
    }
}

