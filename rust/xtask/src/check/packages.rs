//! Rules R1.1 and R1.2: no dependency on a folder outside the workspace, and no build scripts and no procedural macros
//! of the workspace's own. Which libraries a crate uses is weighed in review (decision D22), not checked here.

use super::Workspace;
use crate::problem::Problem;
use crate::workspace::Origin;

/// The problems of the manifests: a dependency on a folder outside the workspace, a build script or a procedural macro.
pub fn check(workspace: &Workspace) -> Vec<Problem> {
    let mut found = Vec::new();
    let member_folders: Vec<&str> = workspace.members.iter().map(|member| member.folder.as_str()).collect();
    for member in &workspace.members {
        let manifest = format!("{}/Cargo.toml", member.folder);
        if member.builds_specially {
            let message = String::from("The crate has a build script or is a procedural macro. Neither is allowed.");
            found.push(Problem::in_file("R1.2", &manifest, message));
        }
        for dependency in &member.dependencies {
            if let Origin::Folder(folder) = &dependency.origin
                && !member_folders.contains(&folder.as_str())
            {
                let message = format!("{} is in {folder}, outside the workspace.", dependency.package);
                found.push(Problem::at_line("R1.1", &manifest, dependency.line, message));
            }
        }
    }
    found
}
