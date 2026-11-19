//! Which hosting service the main remote is on, and the project there (`hosting.detect-remote`).

use super::{HostingRepo, HostingService, KnownHosts, SelfHosted};
use crate::github::main_remote;
use crate::remote::Remote;

/// The kinds `hosting.selfHosted` gives a host name: GitLab, Gitea, and Forgejo, which Gitea's pages serve alike.
pub const SELF_HOSTED_KINDS: [&str; 3] = ["gitlab", "gitea", "forgejo"];

/// The service of a kind of `hosting.selfHosted`; none for a word it does not take.
#[must_use]
pub fn self_hosted_kind(kind: &str) -> Option<HostingService> {
    match kind {
        "gitlab" => Some(HostingService::GitLab),
        "gitea" => Some(HostingService::Gitea),
        "forgejo" => Some(HostingService::Forgejo),
        _ => None,
    }
}

/// The hosting service of the main remote, `origin` when there is one, otherwise the first remote listed, and the
/// project there; nothing when its host name is none Gitardo knows, nor one `hosting.selfHosted` names. On github.com
/// the project is what `github.detect-remote` gives.
///
/// Feature: hosting.detect-remote
#[must_use]
pub fn detect_remote(remotes: &[Remote], known: &KnownHosts) -> Option<HostingRepo> {
    let main = main_remote(remotes)?;
    let url = main.url.trim();
    let parsed = split(url)?;
    let host = parsed.host.to_ascii_lowercase();
    let make = |service, site: String, path: String| Found { service, site, path };
    let found = match host.as_str() {
        "github.com" => {
            let repository = crate::github::detect_remote(remotes)?;
            make(
                HostingService::GitHub,
                String::from("https://github.com"),
                format!("{}/{}", repository.owner, repository.repo),
            )
        }
        "gitlab.com" => make(HostingService::GitLab, site_of(&host), parts(parsed.path, 2..)?),
        "bitbucket.org" => make(HostingService::Bitbucket, site_of(&host), parts(parsed.path, 2..=2)?),
        "codeberg.org" => make(HostingService::Forgejo, site_of(&host), parts(parsed.path, 2..=2)?),
        "dev.azure.com" | "ssh.dev.azure.com" => return on_azure(&parsed, None, main),
        _ if host.ends_with(".visualstudio.com") => {
            let organization = host.strip_suffix(".visualstudio.com").filter(|name| !name.is_empty());
            return on_azure(&parsed, organization, main);
        }
        _ => {
            let own = known.by_host.get(&host)?;
            let wanted = if own.service == HostingService::GitLab {
                2..=usize::MAX
            } else {
                2..=2
            };
            make(own.service, own_site(own, &parsed, &host), parts(parsed.path, wanted)?)
        }
    };
    Some(HostingRepo {
        service: found.service,
        web: format!("{}/{}", found.site, found.path),
        host: host_of(&found.site),
        path: found.path,
        remote: main.name.clone(),
        url: url.to_owned(),
        default_branch: None,
    })
}

/// A service and a project found, before the rest of [`HostingRepo`] is filled in.
struct Found {
    /// The service.
    service: HostingService,
    /// The address of the service: `https://gitlab.com`.
    site: String,
    /// The path of the project there.
    path: String,
}

/// How a URL reaches its host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scheme {
    /// `https://`.
    Https,
    /// `http://`.
    Http,
    /// `ssh://`, or `[user@]host:path`.
    Ssh,
}

/// A URL split into its parts.
#[derive(Debug, Clone, Copy)]
struct Parsed<'a> {
    /// How it is reached.
    scheme: Scheme,
    /// The host name.
    host: &'a str,
    /// The port, when one is written.
    port: Option<&'a str>,
    /// The path, without `/` around it, `.git` taken off.
    path: &'a str,
}

/// `https://[user@]host[:port]/path`, `http://…`, `ssh://[user@]host[:port]/path` and `[user@]host:path`, `.git`
/// and a `/` at the end taken off; nothing for anything else, such as a folder.
fn split(url: &str) -> Option<Parsed<'_>> {
    let (scheme, authority, path) = if let Some((scheme, rest)) = url.split_once("://") {
        let scheme = match scheme.to_ascii_lowercase().as_str() {
            "https" => Scheme::Https,
            "http" => Scheme::Http,
            "ssh" => Scheme::Ssh,
            _ => return None,
        };
        let (authority, path) = rest.split_once('/')?;
        (scheme, authority, path)
    } else {
        let (authority, path) = url.split_once(':')?;
        if authority.contains('/') || authority.contains('\\') {
            return None;
        }
        (Scheme::Ssh, authority, path)
    };
    let host_and_port = authority.rsplit_once('@').map_or(authority, |(_, host)| host);
    let (host, port) = match host_and_port.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (host_and_port, None),
    };
    // One letter before `:` is the drive of a folder on Windows, `C:\…`, not a host.
    if host.len() < 2 {
        return None;
    }
    let path = path.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path).trim_end_matches('/');
    Some(Parsed {
        scheme,
        host,
        port,
        path,
    })
}

/// The path when it has as many parts as wanted, none of them empty.
fn parts(path: &str, wanted: impl std::ops::RangeBounds<usize>) -> Option<String> {
    let count = path.split('/').count();
    (wanted.contains(&count) && path.split('/').all(|part| !part.is_empty())).then(|| path.to_owned())
}

/// The address of a service on its host: `https://gitlab.com`.
fn site_of(host: &str) -> String {
    format!("https://{host}")
}

/// The address of a service of one's own: the one `hosting.selfHosted` gives, else the host of the remote, with its
/// port when the remote is reached on the web; `http` when the remote is.
fn own_site(own: &SelfHosted, parsed: &Parsed<'_>, host: &str) -> String {
    if let Some(web) = &own.web {
        return web.trim_end_matches('/').to_owned();
    }
    let port = parsed.port.filter(|_| parsed.scheme != Scheme::Ssh);
    let scheme = if parsed.scheme == Scheme::Http { "http" } else { "https" };
    match port {
        Some(port) => format!("{scheme}://{host}:{port}"),
        None => format!("{scheme}://{host}"),
    }
}

/// The host name of an address: `git.example.com` for `https://git.example.com:8443/gitlab`.
fn host_of(site: &str) -> String {
    let rest = site.split_once("://").map_or(site, |(_, rest)| rest);
    let authority = rest.split('/').next().unwrap_or(rest);
    authority.split(':').next().unwrap_or(authority).to_ascii_lowercase()
}

/// A repository on Azure DevOps: `dev.azure.com/<org>/<project>/_git/<repo>`, `ssh.dev.azure.com:v3/<org>/<project>/
/// <repo>`, or `<org>.visualstudio.com/[DefaultCollection/]<project>/_git/<repo>`, whose organization is given.
fn on_azure(parsed: &Parsed<'_>, organization: Option<&str>, main: &Remote) -> Option<HostingRepo> {
    let parts: Vec<&str> = parsed.path.split('/').collect();
    let (organization, project, repository) = match (organization, parts.as_slice()) {
        (None, ["v3", organization, project, repository] | [organization, project, "_git", repository]) => {
            (*organization, *project, *repository)
        }
        (Some(organization), ["DefaultCollection", project, "_git", repository] | [project, "_git", repository]) => {
            (organization, *project, *repository)
        }
        _ => return None,
    };
    if [organization, project, repository].iter().any(|part| part.is_empty()) {
        return None;
    }
    Some(HostingRepo {
        service: HostingService::AzureDevOps,
        web: format!("https://dev.azure.com/{organization}/{project}/_git/{repository}"),
        host: String::from("dev.azure.com"),
        path: format!("{organization}/{project}/{repository}"),
        remote: main.name.clone(),
        url: main.url.trim().to_owned(),
        default_branch: None,
    })
}
