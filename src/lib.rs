// Copyright 2018 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

//! get_if_addrs

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/maidsafe/QA/master/Images/
maidsafe_logo.png",
    html_favicon_url = "http://maidsafe.net/img/favicon.ico",
    test(attr(forbid(warnings)))
)]
// For explanation of lint checks, run `rustc -W help` or see
// https://github.com/maidsafe/QA/blob/master/Documentation/Rust%20Lint%20Checks.md
#![forbid(
    arithmetic_overflow,
    mutable_transmutes,
    no_mangle_const_items,
    unknown_crate_types,
    warnings
)]
#![deny(
    bad_style,
    deprecated,
    improper_ctypes,
    missing_docs,
    non_shorthand_field_patterns,
    overflowing_literals,
    stable_features,
    unconditional_recursion,
    unknown_lints,
    unsafe_code,
    unused,
    unused_allocation,
    unused_attributes,
    unused_comparisons,
    unused_features,
    unused_parens,
    while_true
)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    unused_results
)]
#![allow(
    box_pointers,
    missing_copy_implementations,
    missing_debug_implementations,
    variant_size_differences
)]
#![deny(
    clippy::all,
    clippy::unicode_not_nfc,
    clippy::wrong_pub_self_convention,
    clippy::option_unwrap_used
)]
#![allow(clippy::use_debug, clippy::too_many_arguments)]

#[cfg(not(windows))]
mod ifaddrs_posix;
#[cfg(windows)]
mod ifaddrs_windows;
mod sockaddr;

use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
#[cfg(test)]
#[macro_use]
extern crate unwrap;
#[cfg(target_os = "android")]
extern crate get_if_addrs_sys;

/// Details about an interface on this host.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Interface {
    /// The name of the interface.
    pub name: String,
    /// The address details of the interface.
    pub addr: IfAddr,
}

impl Interface {
    /// Check whether this is a loopback interface.
    pub fn is_loopback(&self) -> bool {
        self.addr.is_loopback()
    }

    /// Get the IP address of this interface.
    pub fn ip(&self) -> IpAddr {
        self.addr.ip()
    }
}

/// Details about the address of an interface on this host.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum IfAddr {
    /// This is an Ipv4 interface.
    V4(Ifv4Addr),
    /// This is an Ipv6 interface.
    V6(Ifv6Addr),
}

impl IfAddr {
    /// Check whether this is a loopback address.
    pub fn is_loopback(&self) -> bool {
        match *self {
            IfAddr::V4(ref ifv4_addr) => ifv4_addr.is_loopback(),
            IfAddr::V6(ref ifv6_addr) => ifv6_addr.is_loopback(),
        }
    }

    /// Get the IP address of this interface address.
    pub fn ip(&self) -> IpAddr {
        match *self {
            IfAddr::V4(ref ifv4_addr) => IpAddr::V4(ifv4_addr.ip),
            IfAddr::V6(ref ifv6_addr) => IpAddr::V6(ifv6_addr.ip),
        }
    }
}

/// Details about the ipv4 address of an interface on this host.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Ifv4Addr {
    /// The IP address of the interface.
    pub ip: Ipv4Addr,
    /// The netmask of the interface.
    pub netmask: Ipv4Addr,
    /// The broadcast address of the interface.
    pub broadcast: Option<Ipv4Addr>,
}

impl Ifv4Addr {
    /// Check whether this is a loopback address.
    pub fn is_loopback(&self) -> bool {
        self.ip.octets()[0] == 127
    }
}

/// Details about the ipv6 address of an interface on this host.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Ifv6Addr {
    /// The IP address of the interface.
    pub ip: Ipv6Addr,
    /// The netmask of the interface.
    pub netmask: Ipv6Addr,
    /// The broadcast address of the interface.
    pub broadcast: Option<Ipv6Addr>,
}

impl Ifv6Addr {
    /// Check whether this is a loopback address.
    pub fn is_loopback(&self) -> bool {
        self.ip.segments() == [0, 0, 0, 0, 0, 0, 0, 1]
    }
}

#[cfg(not(windows))]
mod getifaddrs_posix {
    use super::{IfAddr, Ifv4Addr, Ifv6Addr, Interface};
    use crate::ifaddrs_posix::{self as ifaddrs, IfAddrs};
    use crate::sockaddr;
    use std::ffi::CStr;
    use std::io;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    /// Return a vector of IP details for all the valid interfaces on this host.
    #[allow(unsafe_code)]
    pub fn get_if_addrs() -> io::Result<Vec<Interface>> {
        let mut ret = Vec::<Interface>::new();
        let ifaddrs = IfAddrs::new()?;

        for ifaddr in ifaddrs.iter() {
            let addr = match sockaddr::to_ipaddr(ifaddr.ifa_addr) {
                None => continue,
                Some(IpAddr::V4(ipv4_addr)) => {
                    let netmask = match sockaddr::to_ipaddr(ifaddr.ifa_netmask) {
                        Some(IpAddr::V4(netmask)) => netmask,
                        _ => Ipv4Addr::new(0, 0, 0, 0),
                    };
                    let broadcast = if (ifaddr.ifa_flags & 2) != 0 {
                        match ifaddrs::do_broadcast(&ifaddr) {
                            Some(IpAddr::V4(broadcast)) => Some(broadcast),
                            _ => None,
                        }
                    } else {
                        None
                    };

                    IfAddr::V4(Ifv4Addr {
                        ip: ipv4_addr,
                        netmask,
                        broadcast,
                    })
                }
                Some(IpAddr::V6(ipv6_addr)) => {
                    let netmask = match sockaddr::to_ipaddr(ifaddr.ifa_netmask) {
                        Some(IpAddr::V6(netmask)) => netmask,
                        _ => Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0),
                    };
                    let broadcast = if (ifaddr.ifa_flags & 2) != 0 {
                        match ifaddrs::do_broadcast(&ifaddr) {
                            Some(IpAddr::V6(broadcast)) => Some(broadcast),
                            _ => None,
                        }
                    } else {
                        None
                    };

                    IfAddr::V6(Ifv6Addr {
                        ip: ipv6_addr,
                        netmask,
                        broadcast,
                    })
                }
            };

            let name = unsafe { CStr::from_ptr(ifaddr.ifa_name) }
                .to_string_lossy()
                .into_owned();
            ret.push(Interface { name, addr });
        }

        Ok(ret)
    }
}

/// Get a list of all the network interfaces on this machine along with their IP info.
#[cfg(not(windows))]
pub fn get_if_addrs() -> io::Result<Vec<Interface>> {
    getifaddrs_posix::get_if_addrs()
}

#[cfg(windows)]
mod getifaddrs_windows {
    use super::{IfAddr, Ifv4Addr, Ifv6Addr, Interface};
    use crate::ifaddrs_windows::IfAddrs;
    use crate::sockaddr;
    use std::io;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    /// Return a vector of IP details for all the valid interfaces on this host.
    pub fn get_if_addrs() -> io::Result<Vec<Interface>> {
        let mut ret = Vec::<Interface>::new();
        let ifaddrs = IfAddrs::new()?;

        for ifaddr in ifaddrs.iter() {
            for addr in ifaddr.unicast_addresses() {
                let addr = match sockaddr::to_ipaddr(addr.address.lp_socket_address) {
                    None => continue,
                    Some(IpAddr::V4(ipv4_addr)) => {
                        let mut item_netmask = Ipv4Addr::new(0, 0, 0, 0);
                        let mut item_broadcast = None;

                        // Search prefixes for a prefix matching addr
                        'prefixloopv4: for prefix in ifaddr.prefixes() {
                            let ipprefix = sockaddr::to_ipaddr(prefix.address.lp_socket_address);
                            match ipprefix {
                                Some(IpAddr::V4(ref a)) => {
                                    let mut netmask: [u8; 4] = [0; 4];
                                    for (n, netmask_elt) in netmask
                                        .iter_mut()
                                        .enumerate()
                                        .take((prefix.prefix_length as usize + 7) / 8)
                                    {
                                        let x_byte = ipv4_addr.octets()[n];
                                        let y_byte = a.octets()[n];
                                        // Clippy 0.0.128 doesn't handle the label on the `continue`
                                        #[cfg_attr(
                                            feature = "cargo-clippy",
                                            allow(needless_continue)
                                        )]
                                        for m in 0..8 {
                                            if (n * 8) + m > prefix.prefix_length as usize {
                                                break;
                                            }
                                            let bit = 1 << m;
                                            if (x_byte & bit) == (y_byte & bit) {
                                                *netmask_elt |= bit;
                                            } else {
                                                continue 'prefixloopv4;
                                            }
                                        }
                                    }
                                    item_netmask = Ipv4Addr::new(
                                        netmask[0], netmask[1], netmask[2], netmask[3],
                                    );
                                    let mut broadcast: [u8; 4] = ipv4_addr.octets();
                                    for n in 0..4 {
                                        broadcast[n] |= !netmask[n];
                                    }
                                    item_broadcast = Some(Ipv4Addr::new(
                                        broadcast[0],
                                        broadcast[1],
                                        broadcast[2],
                                        broadcast[3],
                                    ));
                                    break 'prefixloopv4;
                                }
                                _ => continue,
                            };
                        }
                        IfAddr::V4(Ifv4Addr {
                            ip: ipv4_addr,
                            netmask: item_netmask,
                            broadcast: item_broadcast,
                        })
                    }
                    Some(IpAddr::V6(ipv6_addr)) => {
                        let mut item_netmask = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0);
                        // Search prefixes for a prefix matching addr
                        'prefixloopv6: for prefix in ifaddr.prefixes() {
                            let ipprefix = sockaddr::to_ipaddr(prefix.address.lp_socket_address);
                            match ipprefix {
                                Some(IpAddr::V6(ref a)) => {
                                    // Iterate the bits in the prefix, if they all match this prefix
                                    // is the right one, else try the next prefix
                                    let mut netmask: [u16; 8] = [0; 8];
                                    for (n, netmask_elt) in netmask
                                        .iter_mut()
                                        .enumerate()
                                        .take((prefix.prefix_length as usize + 15) / 16)
                                    {
                                        let x_word = ipv6_addr.segments()[n];
                                        let y_word = a.segments()[n];
                                        // Clippy 0.0.128 doesn't handle the label on the `continue`
                                        #[cfg_attr(
                                            feature = "cargo-clippy",
                                            allow(needless_continue)
                                        )]
                                        for m in 0..16 {
                                            if (n * 16) + m > prefix.prefix_length as usize {
                                                break;
                                            }
                                            let bit = 1 << m;
                                            if (x_word & bit) == (y_word & bit) {
                                                *netmask_elt |= bit;
                                            } else {
                                                continue 'prefixloopv6;
                                            }
                                        }
                                    }
                                    item_netmask = Ipv6Addr::new(
                                        netmask[0], netmask[1], netmask[2], netmask[3], netmask[4],
                                        netmask[5], netmask[6], netmask[7],
                                    );
                                    break 'prefixloopv6;
                                }
                                _ => continue,
                            };
                        }
                        IfAddr::V6(Ifv6Addr {
                            ip: ipv6_addr,
                            netmask: item_netmask,
                            broadcast: None,
                        })
                    }
                };

                ret.push(Interface {
                    name: ifaddr.name(),
                    addr,
                });
            }
        }

        Ok(ret)
    }
}

#[cfg(windows)]
/// Get address
pub fn get_if_addrs() -> io::Result<Vec<Interface>> {
    getifaddrs_windows::get_if_addrs()
}

#[cfg(test)]
mod tests {
    use super::{get_if_addrs, Interface};
    use std::io::Read;
    use std::net::{IpAddr, Ipv4Addr};
    use std::process::{Command, Stdio};
    use std::str::FromStr;
    use std::thread;
    use std::time::Duration;

    fn list_system_interfaces(cmd: &str, arg: &str) -> String {
        let start_cmd = if arg == "" {
            Command::new(cmd).stdout(Stdio::piped()).spawn()
        } else {
            Command::new(cmd).arg(arg).stdout(Stdio::piped()).spawn()
        };
        let mut process = match start_cmd {
            Err(why) => {
                println!("couldn't start cmd {} : {}", cmd, why);
                return "".to_string();
            }
            Ok(process) => process,
        };
        thread::sleep(Duration::from_millis(1000));
        let _ = process.kill();
        let result: Vec<u8> = unwrap!(process.stdout)
            .bytes()
            .map(|x| unwrap!(x))
            .collect();
        unwrap!(String::from_utf8(result))
    }

    #[cfg(windows)]
    fn list_system_addrs() -> Vec<IpAddr> {
        use std::net::Ipv6Addr;
        list_system_interfaces("ipconfig", "")
            .lines()
            .filter_map(|line| {
                println!("{}", line);
                if line.contains("Address") && !line.contains("Link-local") {
                    let addr_s: Vec<&str> = line.split(" : ").collect();
                    if line.contains("IPv6") {
                        return Some(IpAddr::V6(unwrap!(Ipv6Addr::from_str(addr_s[1]))));
                    } else if line.contains("IPv4") {
                        return Some(IpAddr::V4(unwrap!(Ipv4Addr::from_str(addr_s[1]))));
                    }
                }
                None
            })
            .collect()
    }

    #[cfg(any(target_os = "linux", target_os = "android", target_os = "nacl"))]
    fn list_system_addrs() -> Vec<IpAddr> {
        list_system_interfaces("ip", "addr")
            .lines()
            .filter_map(|line| {
                println!("{}", line);
                if line.contains("inet ") {
                    let addr_s: Vec<&str> = line.split_whitespace().collect();
                    let addr: Vec<&str> = addr_s[1].split('/').collect();
                    return Some(IpAddr::V4(unwrap!(Ipv4Addr::from_str(addr[0]))));
                }
                None
            })
            .collect()
    }

    #[cfg(any(target_os = "freebsd", target_os = "macos", target_os = "ios", target_os = "illumos"))]
    fn list_system_addrs() -> Vec<IpAddr> {
        list_system_interfaces("ifconfig", "")
            .lines()
            .filter_map(|line| {
                println!("{}", line);
                if line.contains("inet ") {
                    let addr_s: Vec<&str> = line.split_whitespace().collect();
                    return Some(IpAddr::V4(unwrap!(Ipv4Addr::from_str(addr_s[1]))));
                }
                None
            })
            .collect()
    }

    #[test]
    fn test_get_if_addrs() {
        let ifaces = unwrap!(get_if_addrs());
        println!("Local interfaces:");
        println!("{:#?}", ifaces);
        // at least one loop back address
        assert!(
            1 <= ifaces
                .iter()
                .filter(|interface| interface.is_loopback())
                .count()
        );
        // one address of IpV4(127.0.0.1)
        let is_loopback =
            |interface: &&Interface| interface.addr.ip() == IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        assert_eq!(1, ifaces.iter().filter(is_loopback).count());

        // each system address shall be listed
        let system_addrs = list_system_addrs();
        assert!(!system_addrs.is_empty());
        for addr in system_addrs {
            let mut listed = false;
            println!("\n checking whether {:?} has been properly listed \n", addr);
            for interface in &ifaces {
                if interface.addr.ip() == addr {
                    listed = true;
                }
            }
            assert!(listed);
        }
    }
}
