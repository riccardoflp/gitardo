//! The proxy requests go through: `HTTPS_PROXY` (or `https_proxy`), otherwise git's `http.proxy`, and the hosts of
//! `NO_PROXY` reached directly (`docs/desktop/05-platform.md`, Proxies).

use crate::error::Error;
use crate::url::{Scheme, Url};

/// Where the proxy settings come from, as found: the variables of the environment and git's configuration. Each is
/// none when it is not set, or set to nothing.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct ProxySettings {
    /// `HTTPS_PROXY`, or `https_proxy` when it is not set.
    pub https_proxy: Option<String>,
    /// What `git config --get http.proxy` gives.
    pub git_http_proxy: Option<String>,
    /// `NO_PROXY`, or `no_proxy` when it is not set.
    pub no_proxy: Option<String>,
}

impl std::fmt::Debug for ProxySettings {
    /// Whether each is set, without its value, which may hold a password.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProxySettings")
            .field("https_proxy", &self.https_proxy.is_some())
            .field("git_http_proxy", &self.git_http_proxy.is_some())
            .field("no_proxy", &self.no_proxy)
            .finish()
    }
}

impl ProxySettings {
    /// The variables of the environment of the program, with what git's `http.proxy` says, which only `git` may
    /// read (rule R1.4) and the caller gives.
    #[must_use]
    pub fn from_environment(git_http_proxy: Option<String>) -> ProxySettings {
        let variable = |upper: &str, lower: &str| {
            std::env::var(upper)
                .ok()
                .filter(|value| !value.trim().is_empty())
                .or_else(|| std::env::var(lower).ok())
                .filter(|value| !value.trim().is_empty())
        };
        ProxySettings {
            https_proxy: variable("HTTPS_PROXY", "https_proxy"),
            git_http_proxy: git_http_proxy.filter(|value| !value.trim().is_empty()),
            no_proxy: variable("NO_PROXY", "no_proxy"),
        }
    }
}

/// Which proxy a request goes through, if any.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Proxies {
    proxy: Option<Url>,
    direct: Vec<String>,
}

impl Proxies {
    /// No proxy: every host is reached directly.
    #[must_use]
    pub fn none() -> Proxies {
        Proxies::default()
    }

    /// The proxy the settings name, `HTTPS_PROXY` before git's: an address such as `http://proxy:3128`, with
    /// `http://` taken when it has no scheme, and a user and a password before its host for basic authentication.
    ///
    /// # Errors
    ///
    /// When the proxy's address cannot be read, or is `https`, which Gitardo does not speak to a proxy.
    pub fn from_settings(settings: &ProxySettings) -> Result<Proxies, Error> {
        let named = settings.https_proxy.as_deref().or(settings.git_http_proxy.as_deref());
        let proxy = named
            .map(|address| {
                let address = address.trim();
                let url = if address.contains("://") {
                    Url::parse(address)?
                } else {
                    Url::parse(&format!("http://{address}"))?
                };
                if url.scheme() == Scheme::Https {
                    return Err(Error::BadUrl {
                        reason: format!("the proxy {} is reached over https, which is not supported", url.host()),
                    });
                }
                Ok(url)
            })
            .transpose()?;
        let direct = settings
            .no_proxy
            .as_deref()
            .unwrap_or_default()
            .split([',', ' '])
            .map(|host| host.trim().to_ascii_lowercase())
            .filter(|host| !host.is_empty())
            .collect();
        Ok(Proxies { proxy, direct })
    }

    /// The proxy a request to an address goes through; none when there is no proxy, or `NO_PROXY` names its host: `*`
    /// for every host, a name for that host and those under it, with a `.` before it or not, and a port after it or
    /// not.
    pub(crate) fn proxy_for(&self, url: &Url) -> Option<&Url> {
        let proxy = self.proxy.as_ref()?;
        let host = url.host();
        let is_direct = self.direct.iter().any(|entry| {
            if entry == "*" {
                return true;
            }
            let (name, port) = match entry.rsplit_once(':') {
                Some((name, port)) if !name.contains(':') && port.parse::<u16>().is_ok() => (name, Some(port)),
                _ => (entry.as_str(), None),
            };
            let name = name.trim_start_matches('.');
            let port_matches = port.is_none_or(|port| port == url.port().to_string());
            let name_matches = host == name || host.strip_suffix(name).is_some_and(|rest| rest.ends_with('.'));
            port_matches && name_matches
        });
        (!is_direct).then_some(proxy)
    }
}

#[cfg(test)]
mod tests {
    use super::{Proxies, ProxySettings};
    use crate::url::Url;

    fn settings(https_proxy: Option<&str>, git_http_proxy: Option<&str>, no_proxy: Option<&str>) -> ProxySettings {
        ProxySettings {
            https_proxy: https_proxy.map(str::to_owned),
            git_http_proxy: git_http_proxy.map(str::to_owned),
            no_proxy: no_proxy.map(str::to_owned),
        }
    }

    fn proxy_of(proxies: &Proxies, address: &str) -> Option<String> {
        proxies
            .proxy_for(&Url::parse(address).unwrap())
            .map(ToString::to_string)
    }

    #[test]
    fn https_proxy_comes_before_the_proxy_of_git() {
        let proxies = Proxies::from_settings(&settings(Some("http://first:3128"), Some("second:8080"), None)).unwrap();
        assert_eq!(
            proxy_of(&proxies, "https://api.github.com/"),
            Some(String::from("http://first:3128/"))
        );
        let proxies = Proxies::from_settings(&settings(None, Some("second:8080"), None)).unwrap();
        assert_eq!(
            proxy_of(&proxies, "https://api.github.com/"),
            Some(String::from("http://second:8080/"))
        );
        let proxies = Proxies::from_settings(&settings(None, None, None)).unwrap();
        assert_eq!(proxy_of(&proxies, "https://api.github.com/"), None);
    }

    #[test]
    fn no_proxy_names_the_hosts_reached_directly() {
        let proxies = Proxies::from_settings(&settings(
            Some("proxy:3128"),
            None,
            Some("localhost, .corp.test,github.com:443"),
        ))
        .unwrap();
        assert_eq!(proxy_of(&proxies, "https://localhost/"), None);
        assert_eq!(proxy_of(&proxies, "https://git.corp.test/"), None);
        assert_eq!(proxy_of(&proxies, "https://corp.test/"), None);
        assert_eq!(proxy_of(&proxies, "https://api.github.com/"), None);
        assert!(proxy_of(&proxies, "https://github.com:8443/").is_some());
        assert!(proxy_of(&proxies, "https://notcorp.test/").is_some());
        assert!(proxy_of(&proxies, "https://example.test/").is_some());
        let everything = Proxies::from_settings(&settings(Some("proxy:3128"), None, Some("*"))).unwrap();
        assert_eq!(proxy_of(&everything, "https://example.test/"), None);
    }

    #[test]
    fn a_proxy_over_https_is_refused() {
        assert!(Proxies::from_settings(&settings(Some("https://proxy:3128"), None, None)).is_err());
    }

    #[test]
    fn the_settings_never_show_a_password() {
        let shown = format!("{:?}", settings(Some("http://ada:secret@proxy:3128"), None, None));
        assert!(!shown.contains("secret"), "{shown}");
    }
}
