//! Enumerasi serial port dan identifikasi USB-UART.
//!
//! Tidak semua serial port adalah USB-UART, jadi kita membedakan jenis port
//! berdasarkan informasi `serialport::SerialPortType`.

use serde::{Deserialize, Serialize};
use serialport::{SerialPortType, UsbPortInfo};

use crate::error::{Error, Result};

/// Jenis port serial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortKind {
    /// Port USB (kemungkinan besar USB-to-UART bridge).
    Usb,
    /// Port hardware bawaan (mis. COM1, /dev/ttyS0).
    Hardware,
    /// Port Bluetooth.
    Bluetooth,
    /// Jenis tidak diketahui.
    Unknown,
}

/// Informasi satu serial port.
///
/// Field USB bersifat opsional karena tidak setiap port adalah USB-UART.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerialPortInfo {
    /// Nama port (mis. `COM7`, `/dev/ttyUSB0`).
    pub name: String,
    /// Jenis port.
    pub kind: PortKind,
    /// USB Vendor ID (bila USB).
    pub vid: Option<u16>,
    /// USB Product ID (bila USB).
    pub pid: Option<u16>,
    /// Nama manufacturer (bila tersedia).
    pub manufacturer: Option<String>,
    /// Nomor seri perangkat (bila tersedia).
    pub serial_number: Option<String>,
    /// Nama produk/device (bila tersedia).
    pub product: Option<String>,
}

impl SerialPortInfo {
    /// Format VID:PID sebagai hex huruf besar, mis. `10C4:EA60`.
    pub fn vid_pid(&self) -> Option<String> {
        match (self.vid, self.pid) {
            (Some(v), Some(p)) => Some(format!("{v:04X}:{p:04X}")),
            _ => None,
        }
    }

    /// Tebakan nama chip USB-UART berdasarkan VID/PID yang dikenal.
    ///
    /// Hanya bersifat informatif; bukan klaim pasti.
    pub fn usb_uart_chip(&self) -> Option<&'static str> {
        let vid = self.vid?;
        let pid = self.pid?;
        Some(match (vid, pid) {
            (0x10C4, 0xEA60) => "CP2102/CP210x",
            (0x1A86, 0x7523) => "CH340",
            (0x1A86, 0x5523) => "CH341",
            (0x0403, 0x6001) => "FT232R",
            (0x0403, 0x6010) => "FT2232",
            (0x0403, 0x6015) => "FT231X",
            (0x067B, 0x2303) => "PL2303",
            (0x2341, _) => "Arduino",
            (0x303A, _) => "Espressif (ESP32)",
            (0x0483, 0x5740) => "STM32 Virtual COM",
            _ => return None,
        })
    }
}

/// Field USB yang diekstrak dari [`UsbPortInfo`].
struct UsbFields {
    vid: Option<u16>,
    pid: Option<u16>,
    manufacturer: Option<String>,
    serial_number: Option<String>,
    product: Option<String>,
}

/// Mengubah [`UsbPortInfo`] dari serialport menjadi field opsional kita.
fn from_usb(usb: &UsbPortInfo) -> UsbFields {
    UsbFields {
        vid: Some(usb.vid),
        pid: Some(usb.pid),
        manufacturer: usb.manufacturer.clone(),
        serial_number: usb.serial_number.clone(),
        product: usb.product.clone(),
    }
}

/// Enumerasi semua serial port yang tersedia di sistem.
pub fn list_ports() -> Result<Vec<SerialPortInfo>> {
    let ports = serialport::available_ports()?;
    let mut out = Vec::with_capacity(ports.len());
    for p in ports {
        let info = match &p.port_type {
            SerialPortType::UsbPort(usb) => {
                let f = from_usb(usb);
                SerialPortInfo {
                    name: p.port_name.clone(),
                    kind: PortKind::Usb,
                    vid: f.vid,
                    pid: f.pid,
                    manufacturer: f.manufacturer,
                    serial_number: f.serial_number,
                    product: f.product,
                }
            }
            SerialPortType::PciPort => SerialPortInfo {
                name: p.port_name.clone(),
                kind: PortKind::Hardware,
                vid: None,
                pid: None,
                manufacturer: None,
                serial_number: None,
                product: None,
            },
            SerialPortType::BluetoothPort => SerialPortInfo {
                name: p.port_name.clone(),
                kind: PortKind::Bluetooth,
                vid: None,
                pid: None,
                manufacturer: None,
                serial_number: None,
                product: None,
            },
            SerialPortType::Unknown => SerialPortInfo {
                name: p.port_name.clone(),
                kind: PortKind::Unknown,
                vid: None,
                pid: None,
                manufacturer: None,
                serial_number: None,
                product: None,
            },
        };
        out.push(info);
    }
    out.sort_by(|a, b| natural_cmp(&a.name, &b.name));
    Ok(out)
}

/// Mencari satu port berdasarkan nama.
pub fn find_port(name: &str) -> Result<SerialPortInfo> {
    list_ports()?
        .into_iter()
        .find(|p| p.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| Error::PortNotFound(name.to_string()))
}

/// Perbandingan natural sederhana agar `COM2` < `COM10`.
fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    fn split(s: &str) -> (String, Option<u64>) {
        let idx = s.find(|c: char| c.is_ascii_digit()).unwrap_or(s.len());
        let (prefix, rest) = s.split_at(idx);
        let num = rest.parse::<u64>().ok();
        (prefix.to_string(), num)
    }
    let (pa, na) = split(a);
    let (pb, nb) = split(b);
    pa.cmp(&pb).then(na.cmp(&nb))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_ordering_com() {
        let mut v = vec!["COM10", "COM2", "COM1", "COM7"];
        v.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(v, vec!["COM1", "COM2", "COM7", "COM10"]);
    }

    #[test]
    fn chip_detection() {
        let info = SerialPortInfo {
            name: "COM7".into(),
            kind: PortKind::Usb,
            vid: Some(0x10C4),
            pid: Some(0xEA60),
            manufacturer: Some("Silicon Labs".into()),
            serial_number: None,
            product: None,
        };
        assert_eq!(info.vid_pid().as_deref(), Some("10C4:EA60"));
        assert_eq!(info.usb_uart_chip(), Some("CP2102/CP210x"));
    }

    #[test]
    fn chip_unknown_returns_none() {
        let info = SerialPortInfo {
            name: "COM1".into(),
            kind: PortKind::Hardware,
            vid: None,
            pid: None,
            manufacturer: None,
            serial_number: None,
            product: None,
        };
        assert_eq!(info.usb_uart_chip(), None);
        assert_eq!(info.vid_pid(), None);
    }
}
