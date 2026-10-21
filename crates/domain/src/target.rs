use std::net::IpAddr;

/// An address probed to measure the connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Target(IpAddr);

impl Target {
    #[must_use]
    pub const fn new(address: IpAddr) -> Self {
        Self(address)
    }

    /// Builds a target from a literal address known at compile time.
    ///
    /// # Panics
    ///
    /// Panics when `address` is not an IP address, which is a programming error.
    #[must_use]
    pub fn parse(address: &str) -> Self {
        Self(address.parse().expect("a literal IP address"))
    }

    /// An address typed by the user, spaces around it ignored; `None` unless it is an IP address.
    /// O(length).
    #[must_use]
    pub fn try_parse(text: &str) -> Option<Self> {
        text.trim().parse().ok().map(Self)
    }

    #[must_use]
    pub const fn is_ipv6(self) -> bool {
        self.0.is_ipv6()
    }

    #[must_use]
    pub const fn address(self) -> IpAddr {
        self.0
    }
}
