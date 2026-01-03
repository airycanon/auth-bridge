use crate::apis::proxy::Proxy;
use http::Uri;
use kube::ResourceExt;

pub trait ProxyFilter {
    fn filter(&self, proxy: &Proxy, uri: &Uri) -> bool;
}

#[derive(Debug, Default, Clone)]
#[allow(dead_code)]
pub struct AddressFilter;
impl ProxyFilter for AddressFilter {
    fn filter(&self, proxy: &Proxy, uri: &Uri) -> bool {
        proxy.spec.address == *uri
    }
}

#[derive(Debug, Default, Clone)]
pub struct NameFilter;
impl ProxyFilter for NameFilter {
    fn filter(&self, proxy: &Proxy, uri: &Uri) -> bool {
        if let Some(authority) = uri.authority() {
            let host = authority.to_string();
            let parts: Vec<&str> = host.split('.').collect();

            let proxy_namespace = proxy.namespace().unwrap_or_default();
            let proxy_name = proxy.name_any();

            println!(
                "filter proxy, host: {}, proxy: {}/{}",
                authority, proxy_namespace, proxy_name
            );

            match parts.as_slice() {
                [name, namespace, ..] => proxy_namespace == *namespace && proxy_name == *name,
                [name] => proxy_namespace == "default" && proxy_name == *name,
                _ => false,
            }
        } else {
            false
        }
    }
}
