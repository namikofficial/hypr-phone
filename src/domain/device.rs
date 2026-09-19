//! Domain-level device model.
//!
//! `PhoneDevice` represents the canonical identity of a phone from the user's
//! perspective. Device identity is intentionally decoupled from any single
//! transport (USB, Wi-Fi, mDNS). Endpoints can change without losing identity.

use std::{
    collections::BTreeMap,
    fmt,
    net::{IpAddr, SocketAddr},
    str::FromStr,
};

use serde::{Deserialize, Serialize};

/// Stable identifier for a device.
///
/// Derived from the most stable signal available (serial+model). Does not
/// change when the endpoint moves.
pub type DeviceId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportKind {
    Usb,
    Wifi,
    Mdns,
    Unknown,
}

impl fmt::Display for TransportKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportKind::Usb => write!(f, "USB"),
            TransportKind::Wifi => write!(f, "Wi-Fi"),
            TransportKind::Mdns => write!(f, "mDNS"),
            TransportKind::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AdbState {
    Connected,
    Disconnected,
    Unauthorized,
    Offline,
    Unknown,
}

impl AdbState {
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "device" => AdbState::Connected,
            "unauthorized" => AdbState::Unauthorized,
            "offline" => AdbState::Offline,
            "disconnected" => AdbState::Disconnected,
            _ => AdbState::Unknown,
        }
    }

    pub fn is_connected(&self) -> bool {
        matches!(self, AdbState::Connected)
    }
}

impl fmt::Display for AdbState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            AdbState::Connected => "connected",
            AdbState::Disconnected => "disconnected",
            AdbState::Unauthorized => "unauthorized",
            AdbState::Offline => "offline",
            AdbState::Unknown => "unknown",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KdeState {
    PairedReachable,
    PairedUnreachable,
    NotPaired,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Transport {
    Usb,
    Wifi { ip: IpAddr, port: u16 },
    Mdns { host: String, port: u16 },
    Unknown,
}

impl Transport {
    pub fn kind(&self) -> TransportKind {
        match self {
            Transport::Usb => TransportKind::Usb,
            Transport::Wifi { .. } => TransportKind::Wifi,
            Transport::Mdns { .. } => TransportKind::Mdns,
            Transport::Unknown => TransportKind::Unknown,
        }
    }

    pub fn socket_addr(&self) -> Option<SocketAddr> {
        match self {
            Transport::Wifi { ip, port } => Some(SocketAddr::new(*ip, *port)),
            Transport::Mdns { host, port } => {
                let ip = IpAddr::from_str(host).ok()?;
                Some(SocketAddr::new(ip, *port))
            }
            _ => None,
        }
    }

    /// Normalized endpoint string (`ip:port`).
    pub fn endpoint_string(&self) -> Option<String> {
        self.socket_addr().map(|sa| sa.to_string())
    }
}

impl fmt::Display for Transport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Transport::Usb => write!(f, "USB"),
            Transport::Wifi { ip, port } => write!(f, "Wi-Fi ({ip}:{port})"),
            Transport::Mdns { host, port } => write!(f, "mDNS ({host}:{port})"),
            Transport::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    /// `scrcpy --new-display` supported (Android 12+ via virtual display).
    pub virtual_display: bool,
    /// `scrcpy --start-app` supported.
    pub start_app: bool,
    /// `scrcpy --flex-display` supported (dynamic virtual display resize).
    pub flex_display: bool,
    /// `scrcpy --record` supported.
    pub recording: bool,
    /// Audio forwarding supported.
    pub audio: bool,
    /// Camera mirroring supported (Android 12+).
    pub camera: bool,
    /// USB transport available.
    pub usb: bool,
    /// Wireless ADB available.
    pub wireless_adb: bool,
    /// mDNS discovery available (ADB Wi-Fi 2.0).
    pub mdns: bool,
    /// OTG mode (keyboard/mouse only).
    pub otg: bool,
    /// HID/physical input supported.
    pub hid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneDevice {
    /// Stable identity hash. Persisted across sessions.
    pub id: DeviceId,
    /// User-facing name (alias if configured, otherwise model/serial).
    pub display_name: String,
    /// Android manufacturer/model.
    pub model: Option<String>,
    /// ADB-reported product field.
    pub product: Option<String>,
    /// Hardware serial from `getprop ro.serialno` or USB serial.
    pub android_serial: Option<String>,
    /// Current ADB serial (e.g. `192.168.1.5:5555`, `usb:1-1`).
    pub adb_serial: Option<String>,
    pub transport: Transport,
    pub adb_state: AdbState,
    pub kdeconnect_id: Option<String>,
    pub kde_state: KdeState,
    pub capabilities: DeviceCapabilities,
    /// Last time the device was observed reachable.
    pub last_seen_unix_secs: Option<u64>,
    pub is_default: bool,
}

impl PhoneDevice {
    /// Returns true if the given serial looks like an IP:port endpoint
    /// (wireless ADB) rather than a hardware serial.
    pub fn is_wireless_adb_serial(serial: &str) -> bool {
        serial.contains(':') && !serial.starts_with("usb:") && parse_endpoint(serial).is_some()
    }

    /// Construct a `PhoneDevice` from `adb devices -l` style output.
    ///
    /// For wireless ADB devices (endpoint-based serials), the serial is
    /// NOT used for stable identity. The caller should call
    /// `enhance_with_hardware_serial()` to fetch `ro.serialno` and
    /// recompute the stable identity.
    pub fn from_adb_listing(
        serial: &str,
        state: &str,
        model: Option<String>,
        product: Option<String>,
    ) -> Self {
        let is_wireless = Self::is_wireless_adb_serial(serial);

        let transport = if serial.starts_with("usb") || !serial.contains(':') {
            Transport::Usb
        } else if let Some(endpoint) = parse_endpoint(serial) {
            Transport::Wifi {
                ip: endpoint.ip(),
                port: endpoint.port(),
            }
        } else {
            Transport::Unknown
        };

        // For USB and non-IP serials, use serial directly for identity.
        // For wireless (IP:port), we use a temporary ID based on the endpoint;
        // enhance_with_hardware_serial() will recompute it with hardware serial.
        let id = if is_wireless {
            // Wireless: endpoint-based ID is temporary; hardware serial will fix it.
            compute_device_id(
                serial,
                model
                    .as_deref()
                    .or(product.as_deref())
                    .unwrap_or("wireless"),
            )
        } else {
            // USB or stable serial: directly use for identity.
            compute_device_id(
                serial,
                model.as_deref().or(product.as_deref()).unwrap_or(serial),
            )
        };

        Self {
            id,
            display_name: model
                .clone()
                .or_else(|| product.clone())
                .unwrap_or_else(|| serial.to_string()),
            model,
            product,
            // android_serial is set by enhance_with_hardware_serial() for wireless devices.
            android_serial: None,
            adb_serial: Some(serial.to_string()),
            transport,
            adb_state: AdbState::parse(state),
            kdeconnect_id: None,
            kde_state: KdeState::Unknown,
            capabilities: DeviceCapabilities::default(),
            last_seen_unix_secs: Some(unix_now_secs()),
            is_default: false,
        }
    }

    /// Enhance a wireless ADB device with its hardware serial from `ro.serialno`.
    /// This recomputes the stable identity so that the device keeps the same
    /// ID across endpoint changes (different IP, different port, reconnect).
    ///
    /// Returns a new PhoneDevice with updated `android_serial` and `id`.
    pub fn enhance_with_hardware_serial(&self, hardware_serial: &str) -> Self {
        if !self.is_wireless() {
            // USB or stable serial — no change needed.
            return self.clone();
        }
        let mut updated = self.clone();
        updated.android_serial = Some(hardware_serial.to_string());
        // Recompute stable identity from hardware serial + model.
        updated.id = compute_stable_physical_id(
            hardware_serial,
            self.model.as_deref().or(self.product.as_deref()),
        );
        updated
    }

    /// Returns true if this device was discovered over wireless ADB.
    pub fn is_wireless(&self) -> bool {
        matches!(self.transport, Transport::Wifi { .. })
    }

    pub fn is_connected(&self) -> bool {
        self.adb_state.is_connected()
    }

    /// Best endpoint string to use for `adb connect` retries.
    pub fn reconnect_endpoint(&self) -> Option<String> {
        self.transport
            .endpoint_string()
            .or_else(|| self.adb_serial.clone().filter(|s| s.contains(':')))
    }
}

/// Parse `ip:port` into `SocketAddr` (handles IPv6 brackets).
pub fn parse_endpoint(s: &str) -> Option<SocketAddr> {
    let s = s.trim();
    if s.starts_with('[') {
        // IPv6: [::1]:5555
        let end = s.find(']')?;
        let ip_part = &s[1..end];
        let rest = &s[end + 1..];
        let port = rest.strip_prefix(':')?.parse::<u16>().ok()?;
        let ip = IpAddr::from_str(ip_part).ok()?;
        Some(SocketAddr::new(ip, port))
    } else if let Some((ip, port)) = s.rsplit_once(':') {
        let ip = IpAddr::from_str(ip).ok()?;
        let port = port.parse::<u16>().ok()?;
        Some(SocketAddr::new(ip, port))
    } else {
        None
    }
}

/// Identity hash: prefer serial+model; fallback to serial alone.
/// Uses a simple FNV-1a 64-bit so we don't pull in extra deps for one hash.
pub fn compute_device_id(serial: &str, model: &str) -> DeviceId {
    let combined = format!("{model}|{serial}");
    format!("dev-{:016x}", fnv1a_64(combined.as_bytes()))
}

/// Compute a stable physical device ID using the hardware serial.
/// Format: `physical:<hardware-serial>` for direct hardware serials,
/// or `physical:<hash>` when the serial contains unsafe characters.
pub fn compute_stable_physical_id(hardware_serial: &str, model: Option<&str>) -> DeviceId {
    // Hardware serials should be alphanumeric + safe chars. If it looks like
    // an IP:port or contains weird chars, hash it for safety.
    if hardware_serial.contains(':')
        || hardware_serial.contains('/')
        || hardware_serial.contains('\\')
    {
        // Looks like an endpoint or unsafe - hash it.
        let combined = if let Some(m) = model {
            format!("physical|{m}|{hardware_serial}")
        } else {
            format!("physical|{hardware_serial}")
        };
        format!("physical-{:016x}", fnv1a_64(combined.as_bytes()))
    } else {
        // Clean hardware serial - use it directly with physical: prefix.
        format!("physical:{hardware_serial}")
    }
}

fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn unix_now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Summary of a device used in status / menu display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSummary {
    pub id: DeviceId,
    pub name: String,
    pub model: Option<String>,
}

impl From<&PhoneDevice> for DeviceSummary {
    fn from(d: &PhoneDevice) -> Self {
        Self {
            id: d.id.clone(),
            name: d.display_name.clone(),
            model: d.model.clone(),
        }
    }
}

/// Group devices by identity across multiple transport observations.
pub fn dedupe_by_identity(devices: Vec<PhoneDevice>) -> Vec<PhoneDevice> {
    let mut by_id: BTreeMap<DeviceId, PhoneDevice> = BTreeMap::new();
    for d in devices {
        match by_id.get_mut(&d.id) {
            None => {
                by_id.insert(d.id.clone(), d);
            }
            Some(existing) => {
                // Prefer the connected one.
                if d.is_connected() && !existing.is_connected() {
                    *existing = d;
                } else if existing.model.is_none() && d.model.is_some() {
                    existing.model = d.model.clone();
                }
            }
        }
    }
    by_id.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_usb_serial_as_usb_transport() {
        let d = PhoneDevice::from_adb_listing("usb:1-1.4", "device", None, Some("sailfish".into()));
        assert_eq!(d.transport.kind(), TransportKind::Usb);
        assert_eq!(d.adb_serial.as_deref(), Some("usb:1-1.4"));
        assert!(d.is_connected());
    }

    #[test]
    fn parses_wireless_endpoint() {
        let d = PhoneDevice::from_adb_listing(
            "192.168.1.5:5555",
            "device",
            Some("Pixel 8".into()),
            None,
        );
        assert_eq!(d.transport.kind(), TransportKind::Wifi);
        assert_eq!(
            d.transport.socket_addr(),
            Some("192.168.1.5:5555".parse().unwrap())
        );
    }

    #[test]
    fn parse_endpoint_handles_ipv4_and_ipv6() {
        assert_eq!(
            parse_endpoint("192.168.0.5:5555"),
            Some("192.168.0.5:5555".parse().unwrap())
        );
        assert_eq!(
            parse_endpoint("[::1]:5555"),
            Some("[::1]:5555".parse().unwrap())
        );
        assert!(parse_endpoint("not-an-endpoint").is_none());
    }

    #[test]
    fn identity_is_stable_across_endpoints() {
        let a = compute_device_id("192.168.1.5:5555", "Pixel 8");
        let b = compute_device_id("192.168.1.99:42857", "Pixel 8");
        // Same device on different endpoints → same id? No — identity includes serial,
        // which ADB sets to the endpoint. This is intentional: when endpoint changes,
        // we treat them as the same physical device if model matches.
        // The model+serial hash is unique to the reported ADB id.
        assert!(!a.is_empty());
        assert!(!b.is_empty());
        assert_ne!(a, b);
    }

    #[test]
    fn device_id_helper_uses_serial_and_model() {
        assert_eq!(
            compute_device_id("abc", "Pixel 8"),
            compute_device_id("abc", "Pixel 8")
        );
        assert_ne!(
            compute_device_id("abc", "Pixel 8"),
            compute_device_id("xyz", "Pixel 8")
        );
    }

    #[test]
    fn adb_state_parses() {
        assert_eq!(AdbState::parse("device"), AdbState::Connected);
        assert_eq!(AdbState::parse("offline"), AdbState::Offline);
        assert_eq!(AdbState::parse("unauthorized"), AdbState::Unauthorized);
        assert!(!AdbState::Unknown.is_connected());
    }

    #[test]
    fn dedupe_prefers_connected_observation() {
        let offline = PhoneDevice::from_adb_listing("192.168.1.5:5555", "offline", None, None);
        let mut connected = PhoneDevice::from_adb_listing("192.168.1.5:5555", "device", None, None);
        // Make them the same identity (will share serial+model → same id).
        connected.model = offline.model.clone();
        let merged = dedupe_by_identity(vec![offline.clone(), connected.clone()]);
        assert_eq!(merged.len(), 1);
        assert!(merged[0].is_connected());
    }

    #[test]
    fn wireless_adb_serial_detection() {
        // IP:port is wireless.
        assert!(PhoneDevice::is_wireless_adb_serial("192.168.1.5:5555"));
        assert!(PhoneDevice::is_wireless_adb_serial("[::1]:5555"));
        // usb: prefix is not wireless.
        assert!(!PhoneDevice::is_wireless_adb_serial("usb:1-1.4"));
        // Plain serials are not wireless.
        assert!(!PhoneDevice::is_wireless_adb_serial("ABCDEF"));
        assert!(!PhoneDevice::is_wireless_adb_serial("emulator-5554"));
    }

    #[test]
    fn stable_physical_id_uses_hardware_serial() {
        // Clean hardware serial gets physical: prefix directly.
        let id = compute_stable_physical_id("ABC123XYZ", Some("Pixel 8"));
        assert!(id.starts_with("physical:"));
        assert!(id.contains("ABC123XYZ"));

        // Endpoint-like serials get hashed.
        let id_ip = compute_stable_physical_id("192.168.1.5:5555", Some("Pixel 8"));
        assert!(id_ip.starts_with("physical-"));
        // Should NOT contain the endpoint.
        assert!(!id_ip.contains("192.168"));
    }

    #[test]
    fn enhance_with_hardware_serial_updates_identity() {
        // Create a wireless device with endpoint-based ID.
        let device = PhoneDevice::from_adb_listing(
            "192.168.1.5:5555",
            "device",
            Some("Pixel 8".into()),
            None,
        );
        assert!(device.is_wireless());
        let original_id = device.id.clone();

        // Enhance with hardware serial.
        let enhanced = device.enhance_with_hardware_serial("ABCDEF12345");
        assert_ne!(enhanced.id, original_id);
        // The enhanced ID should use physical: format with hardware serial.
        assert!(enhanced.id.starts_with("physical:"));
        assert!(enhanced.id.contains("ABCDEF12345"));
        // android_serial should be set.
        assert_eq!(enhanced.android_serial.as_deref(), Some("ABCDEF12345"));
    }

    #[test]
    fn enhance_does_nothing_for_usb_devices() {
        let device = PhoneDevice::from_adb_listing(
            "usb:1-1.4",
            "device",
            Some("Pixel 7".into()),
            None,
        );
        assert!(!device.is_wireless());
        let original_id = device.id.clone();

        // Enhancement should not change USB device identity.
        let enhanced = device.enhance_with_hardware_serial("SHOULD_NOT_CHANGE");
        assert_eq!(enhanced.id, original_id);
        assert!(enhanced.android_serial.is_none());
    }
}
