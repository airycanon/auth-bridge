use crate::apis::proxy::Proxy;
use http::Uri;
use kube::ResourceExt;

pub trait ProxyFilter {
    fn filter(&self, proxy: &Proxy, uri: &Uri) -> bool;
}

pub struct AddressFilter;
impl ProxyFilter for AddressFilter {
    fn filter(&self, proxy: &Proxy, uri: &Uri) -> bool {
        proxy.spec.address == *uri
    }
}

pub struct NameFilter;
impl ProxyFilter for NameFilter {
    fn filter(&self, proxy: &Proxy, uri: &Uri) -> bool {
        let host = uri.host().unwrap_or_default();
        let proxy_name = match host.split('.').next() {
            Some(name) => name,
            None => return false,
        };

        proxy.name_any() == proxy_name
    }
}
