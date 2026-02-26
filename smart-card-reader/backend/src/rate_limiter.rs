//! Rate limiting module for WebSocket connections
//!
//! Implements token bucket algorithm to prevent abuse and ensure fair resource allocation.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum number of requests allowed in the window
    pub max_requests: u32,
    /// Time window for rate limiting
    pub window: Duration,
    /// Maximum concurrent connections per IP
    pub max_connections: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 60,                // 60 requests per window
            window: Duration::from_secs(60), // 1 minute window
            max_connections: 5,              // 5 concurrent connections per IP
        }
    }
}

/// Rate limit state for a single IP address
#[derive(Debug, Clone)]
struct RateLimitState {
    /// Number of tokens available (requests allowed)
    tokens: u32,
    /// Last time tokens were refilled
    last_refill: Instant,
    /// Number of active connections
    active_connections: u32,
}

impl RateLimitState {
    fn new(max_tokens: u32) -> Self {
        Self {
            tokens: max_tokens,
            last_refill: Instant::now(),
            active_connections: 0,
        }
    }
}

/// Rate limiter using token bucket algorithm
pub struct RateLimiter {
    config: RateLimitConfig,
    states: RwLock<HashMap<IpAddr, RateLimitState>>,
}

impl RateLimiter {
    /// Create a new rate limiter with the given configuration
    #[must_use]
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            states: RwLock::new(HashMap::new()),
        }
    }

    /// Create a rate limiter with default configuration
    #[must_use]
    #[cfg(test)]
    pub fn default_config() -> Self {
        Self::new(RateLimitConfig::default())
    }

    /// Check if a request from the given IP is allowed
    ///
    /// Returns `true` if the request is allowed, `false` if rate limited
    pub fn check_request(&self, ip: IpAddr) -> bool {
        let mut states = self.states.write();

        let state = states
            .entry(ip)
            .or_insert_with(|| RateLimitState::new(self.config.max_requests));

        // Refill tokens based on elapsed time
        let elapsed = state.last_refill.elapsed();
        if elapsed >= self.config.window {
            state.tokens = self.config.max_requests;
            state.last_refill = Instant::now();
        }

        // Check if tokens available
        if state.tokens > 0 {
            state.tokens -= 1;
            true
        } else {
            log::warn!("⚠️ Rate limit exceeded for IP: {}", ip);
            false
        }
    }

    /// Check if a new connection from the given IP is allowed
    ///
    /// Returns `true` if the connection is allowed, `false` if limit exceeded
    pub fn check_connection(&self, ip: IpAddr) -> bool {
        let mut states = self.states.write();

        let state = states
            .entry(ip)
            .or_insert_with(|| RateLimitState::new(self.config.max_requests));

        if state.active_connections < self.config.max_connections {
            state.active_connections += 1;
            log::debug!(
                "✓ Connection allowed for {}: {}/{}",
                ip,
                state.active_connections,
                self.config.max_connections
            );
            true
        } else {
            log::warn!(
                "⚠️ Connection limit exceeded for IP: {} ({} active)",
                ip,
                state.active_connections
            );
            false
        }
    }

    /// Release a connection slot for the given IP
    pub fn release_connection(&self, ip: IpAddr) {
        let mut states = self.states.write();

        if let Some(state) = states.get_mut(&ip) {
            if state.active_connections > 0 {
                state.active_connections -= 1;
                log::debug!(
                    "✓ Connection released for {}: {}/{}",
                    ip,
                    state.active_connections,
                    self.config.max_connections
                );
            }
        }
    }

    /// Clean up expired entries to prevent memory leak
    ///
    /// Removes entries that haven't been accessed for longer than the cleanup threshold
    pub fn cleanup(&self, threshold: Duration) {
        let mut states = self.states.write();
        let now = Instant::now();

        states.retain(|ip, state| {
            let keep =
                state.active_connections > 0 || now.duration_since(state.last_refill) < threshold;

            if !keep {
                log::debug!("🗑️ Cleaned up rate limit state for {}", ip);
            }
            keep
        });
    }

    /// Get current statistics for monitoring
    #[must_use]
    pub fn get_stats(&self) -> RateLimitStats {
        let states = self.states.read();

        RateLimitStats {
            tracked_ips: states.len(),
            total_active_connections: states.values().map(|s| s.active_connections).sum(),
        }
    }
}

/// Rate limiter statistics for monitoring
#[derive(Debug, Clone)]
pub struct RateLimitStats {
    /// Number of IP addresses being tracked
    pub tracked_ips: usize,
    /// Total active connections across all IPs
    pub total_active_connections: u32,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_rate_limiting() {
        let config = RateLimitConfig {
            max_requests: 3,
            window: Duration::from_secs(60),
            max_connections: 2,
        };
        let limiter = RateLimiter::new(config);
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        // First 3 requests should succeed
        assert!(limiter.check_request(ip));
        assert!(limiter.check_request(ip));
        assert!(limiter.check_request(ip));

        // 4th request should fail
        assert!(!limiter.check_request(ip));
    }

    #[test]
    fn test_connection_limiting() {
        let config = RateLimitConfig {
            max_requests: 100,
            window: Duration::from_secs(60),
            max_connections: 2,
        };
        let limiter = RateLimiter::new(config);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        // First 2 connections should succeed
        assert!(limiter.check_connection(ip));
        assert!(limiter.check_connection(ip));

        // 3rd connection should fail
        assert!(!limiter.check_connection(ip));

        // Release one connection
        limiter.release_connection(ip);

        // Now another connection should succeed
        assert!(limiter.check_connection(ip));
    }

    #[test]
    fn test_cleanup() {
        let limiter = RateLimiter::default_config();
        let ip1 = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));

        // Create entries
        limiter.check_request(ip1);
        limiter.check_connection(ip2);

        assert_eq!(limiter.get_stats().tracked_ips, 2);

        // Release connection
        limiter.release_connection(ip2);

        // Cleanup with very short threshold
        limiter.cleanup(Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(10));

        // Both should be cleaned up
        limiter.cleanup(Duration::from_millis(1));
        assert_eq!(limiter.get_stats().tracked_ips, 0);
    }
}
