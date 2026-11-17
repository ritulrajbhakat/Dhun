use std::net::IpAddr;

/// Returns the best local IPv4/IPv6 string (falling back to 127.0.0.1 for desktop compatibility).
pub fn get_local_ip() -> String {
    detect_reachable_lan_ip().unwrap_or_else(|| "127.0.0.1".to_string())
}

/// Detects a genuine reachable private RFC1918 LAN IPv4 address for guest QR codes.
/// Strictly rejects loopback (127.x), unspecified (0.0.0.0), and link-local (169.254.x) addresses.
/// Never fabricates a fake IP address when the host machine is not connected to a LAN.
pub fn detect_reachable_lan_ip() -> Option<String> {
    if let Ok(ip) = local_ip_address::local_ip() {
        if is_reachable_lan_ipv4(ip) {
            return Some(ip.to_string());
        }
    }
    if let Ok(ifaces) = local_ip_address::list_afinet_netifas() {
        for (_name, ip) in ifaces {
            if is_reachable_lan_ipv4(ip) {
                return Some(ip.to_string());
            }
        }
    }
    None
}

/// Returns true only for routable private RFC1918 IPv4 LAN addresses.
pub fn is_reachable_lan_ipv4(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private() && !v4.is_loopback() && !v4.is_link_local() && !v4.is_unspecified(),
        IpAddr::V6(_) => false,
    }
}

/// Strictly validates that an incoming peer socket address belongs to the host's loopback
/// or local private network (RFC1918 IPv4, IPv6 ULA fc00::/7, link-local fe80::/10, or ::1).
/// Rejects public internet IPs so guests outside the host's LAN cannot access the local room.
pub fn is_lan_or_loopback_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_loopback() || v4.is_private() || v4.is_link_local(),
        IpAddr::V6(v6) => {
            if v6.is_loopback() {
                return true;
            }
            // Handle IPv4-mapped IPv6 addresses (::ffff:192.168.x.x)
            if let Some(mapped_v4) = v6.to_ipv4_mapped() {
                return mapped_v4.is_loopback() || mapped_v4.is_private() || mapped_v4.is_link_local();
            }
            let seg0 = v6.segments()[0];
            // Unique Local Address (fc00::/7) or Link-Local (fe80::/10)
            (seg0 & 0xfe00) == 0xfc00 || (seg0 & 0xffc0) == 0xfe80
        }
    }
}

/// Validates HTTP `Host` header against DNS rebinding attacks.
/// Only permits localhost, 127.0.0.1, [::1], or the host machine's actual detected LAN IP.
pub fn is_allowed_local_host_header(host_header: &str, detected_lan_ip: Option<&str>) -> bool {
    let clean = host_header.trim().to_lowercase();
    if clean.is_empty() {
        return false;
    }

    // Strip optional :port while respecting IPv6 [::1]:port
    let host_only = if clean.starts_with('[') {
        clean
            .split(']')
            .next()
            .map(|s| format!("{}]", s))
            .unwrap_or(clean.clone())
    } else {
        clean.split(':').next().unwrap_or(&clean).to_string()
    };

    if host_only == "localhost" || host_only == "127.0.0.1" || host_only == "[::1]" {
        return true;
    }

    if let Some(lan_ip) = detected_lan_ip {
        if host_only == lan_ip.to_lowercase() {
            return true;
        }
    }

    false
}
