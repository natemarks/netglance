//! DNS resolution utilities for caching resolved addresses.

use std::net::{SocketAddr, ToSocketAddrs};
use tracing::{debug, warn};

/// Resolve a host:port address to an IP address, preferring IPv4.
///
/// If the address is already an IP, returns it as-is.
/// If it's a hostname, performs DNS lookup and returns the first IPv4 address
/// if available, otherwise falls back to IPv6.
///
/// # Arguments
/// * `address` - Host:port string (e.g., "example.com:443" or "192.168.1.1:443")
///
/// # Returns
/// - `Ok(String)` - The resolved IP:port address
/// - `Err(String)` - Error message if DNS resolution fails
#[allow(dead_code)] // Kept for potential future use
pub fn resolve_address(address: &str) -> Result<String, String> {
    debug!("Resolving address: {}", address);

    // Try to resolve the address
    match address.to_socket_addrs() {
        Ok(addrs) => {
            // Collect all addresses
            let all_addrs: Vec<SocketAddr> = addrs.collect();

            if all_addrs.is_empty() {
                let err = format!("DNS resolution returned no addresses for: {}", address);
                warn!("{}", err);
                return Err(err);
            }

            // Prefer IPv4 addresses over IPv6
            let preferred_addr = all_addrs
                .iter()
                .find(|addr| addr.is_ipv4())
                .or_else(|| all_addrs.first())
                .unwrap(); // Safe because we checked is_empty above

            let resolved = preferred_addr.to_string();
            debug!(
                "Resolved {} to {} (from {} candidate{})",
                address,
                resolved,
                all_addrs.len(),
                if all_addrs.len() == 1 { "" } else { "s" }
            );

            if all_addrs.len() > 1 {
                debug!(
                    "  Available addresses: {}",
                    all_addrs
                        .iter()
                        .map(|a| a.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }

            Ok(resolved)
        }
        Err(e) => {
            let err = format!("Failed to resolve {}: {}", address, e);
            warn!("{}", err);
            Err(err)
        }
    }
}

/// Check if an address string is already an IP address (not a hostname).
///
/// # Arguments
/// * `address` - Host:port string
///
/// # Returns
/// `true` if the address is already an IP, `false` if it's a hostname
#[allow(dead_code)]
pub fn is_ip_address(address: &str) -> bool {
    // Split off the port
    if let Some((host, _port)) = address.rsplit_once(':') {
        // Try to parse as IP
        host.parse::<std::net::IpAddr>().is_ok()
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ip_address() {
        assert!(is_ip_address("192.168.1.1:443"));
        assert!(is_ip_address("8.8.8.8:53"));
        assert!(is_ip_address("::1:443"));
        assert!(is_ip_address("2001:4860:4860::8888:53"));

        assert!(!is_ip_address("example.com:443"));
        assert!(!is_ip_address("localhost:8080"));
        assert!(!is_ip_address("www.google.com:443"));
    }

    #[test]
    fn test_resolve_ip_address() {
        // Resolving an IP should return the same IP
        let result = resolve_address("8.8.8.8:53");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "8.8.8.8:53");
    }

    #[test]
    fn test_resolve_localhost() {
        // Resolving localhost should work and prefer IPv4
        let result = resolve_address("localhost:8080");
        assert!(result.is_ok());
        let resolved = result.unwrap();
        // Should prefer IPv4 (127.0.0.1) over IPv6 ([::1])
        assert!(
            resolved.starts_with("127.0.0.1:"),
            "Expected IPv4 address, got: {}",
            resolved
        );
    }

    #[test]
    fn test_resolve_invalid_hostname() {
        // Invalid hostname should fail
        let result = resolve_address("invalid.hostname.that.does.not.exist.local:80");
        assert!(result.is_err());
    }

    #[test]
    fn test_ipv4_preference() {
        // When both IPv4 and IPv6 are available, IPv4 should be preferred
        // localhost typically resolves to both 127.0.0.1 and ::1
        let result = resolve_address("localhost:9999");
        assert!(result.is_ok());
        let resolved = result.unwrap();

        // Check if it's IPv4 (doesn't contain brackets which indicate IPv6)
        assert!(
            !resolved.contains('['),
            "Expected IPv4 (no brackets), got IPv6: {}",
            resolved
        );
        assert!(
            resolved.starts_with("127.0.0.1:"),
            "Expected 127.0.0.1, got: {}",
            resolved
        );
    }

    #[test]
    fn test_ipv6_fallback() {
        // Test that IPv6 works when explicitly provided
        let result = resolve_address("[::1]:8080");
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert!(resolved.contains("::1"), "Expected IPv6 address with ::1");
    }
}
