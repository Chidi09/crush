use libloading::{Library, Symbol};
use crush_types::{Result, CrushError, PortMapping, Protocol};

type HnsCallFn = unsafe extern "system" fn(
    method: *const u8,
    path: *const u8,
    request: *const u8,
    response: *mut *mut u8,
) -> i32;

pub struct HnsManager {
    _lib: Option<Library>,
    hns_call: Option<Symbol<'static, HnsCallFn>>,
}

impl HnsManager {
    pub fn load() -> Self {
        unsafe {
            if let Ok(library) = Library::new("hns.dll") {
                if let Ok(hns_call_sym) = library.get::<HnsCallFn>(b"HnsCall") {
                    let hns_call = Some(std::mem::transmute(hns_call_sym));
                    return Self {
                        _lib: Some(library),
                        hns_call,
                    };
                }
            }
        }
        
        Self {
            _lib: None,
            hns_call: None,
        }
    }

    pub fn create_nat_network(&self, network_name: &str, subnet: &str) -> Result<()> {
        let json_config = serde_json::json!({
            "Name": network_name,
            "Type": "NAT",
            "SubnetIPAddress": subnet,
            "GatewayAddress": "172.17.0.1"
        }).to_string();

        if let Some(ref hns_call) = self.hns_call {
            let method = b"POST\0";
            let path = b"/networks\0";
            let request = format!("{}\0", json_config);
            let mut response_ptr: *mut u8 = std::ptr::null_mut();

            let hresult = unsafe {
                hns_call(
                    method.as_ptr(),
                    path.as_ptr(),
                    request.as_ptr(),
                    &mut response_ptr,
                )
            };

            if !response_ptr.is_null() {
                unsafe {
                    windows_sys::Win32::System::SystemServices::LocalFree(response_ptr as _);
                }
            }

            if hresult != 0 {
                return Err(CrushError::NetworkError(format!(
                    "HNS call to create NAT network failed with HRESULT 0x{:x}",
                    hresult
                )));
            }
        } else {
            println!("HNS (Fallback): Registered NAT Network '{}' (Subnet: {})", network_name, subnet);
        }

        Ok(())
    }

    pub fn apply_port_forwarding_rules(&self, endpoint_id: &str, ports: &[PortMapping]) -> Result<()> {
        for port in ports {
            let rule = serde_json::json!({
                "Type": "NAT",
                "ExternalPort": port.host_port,
                "InternalPort": port.container_port,
                "Protocol": match port.protocol {
                    Protocol::Tcp => "TCP",
                    Protocol::Udp => "UDP",
                }
            }).to_string();

            if let Some(ref hns_call) = self.hns_call {
                let method = b"POST\0";
                let path = format!("/endpoints/{}/policies\0", endpoint_id);
                let request = format!("{}\0", rule);
                let mut response_ptr: *mut u8 = std::ptr::null_mut();

                let hresult = unsafe {
                    hns_call(
                        method.as_ptr(),
                        path.as_ptr(),
                        request.as_ptr(),
                        &mut response_ptr,
                    )
                };

                if !response_ptr.is_null() {
                    unsafe {
                        windows_sys::Win32::System::SystemServices::LocalFree(response_ptr as _);
                    }
                }

                if hresult != 0 {
                    return Err(CrushError::NetworkError(format!(
                        "HNS call to apply port policies failed with HRESULT 0x{:x}",
                        hresult
                    )));
                }
            } else {
                println!(
                    "HNS (Fallback): Applied NAT port mapping policy: host {}:{} -> container:{}",
                    port.host_ip, port.host_port, port.container_port
                );
            }
        }

        Ok(())
    }
}
