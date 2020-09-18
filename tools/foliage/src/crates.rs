//! Crates, as understood in the RedLeaf universe

use std::path::PathBuf;
use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use super::project::Project;

#[derive(Debug)]
pub struct Crate {
    pub name: String,
    pub canonical_path: PathBuf,
    pub crate_type: CrateType,
    pub crate_scope: CrateScope,
    pub dependencies: Vec<Dependency>,
}

#[derive(Debug)]
pub enum Dependency {
    Local(LocalDependency),
    Remote(RemoteDependency),
    Illegal(IllegalDependency),
}

#[derive(Debug)]
pub struct LocalDependency {
    pub name: String,

    /// Canonical path to the local dependency
    pub canonical_path: Option<PathBuf>,

    /// Whether the dependency can be resolved
    ///
    /// Always false if it refers to a path outside
    /// of the project root.
    pub is_resolvable: bool,
}

#[derive(Debug)]
pub struct RemoteDependency {
    pub name: String,
}

#[derive(Debug)]
pub struct IllegalDependency {
    pub name: String,
}

#[non_exhaustive]
#[derive(Debug)]
pub enum CrateType {
    /// A domain
    Domain,

    /// A library
    Library,

    /// The kernel
    Kernel,
}

#[derive(Debug)]
pub enum CrateScope {
    /// Can be only depended on in domains
    Domains,

    /// Can be only depended on in the kernel
    Kernel,

    /// Can be depended on by either the kernel or any domain
    Shared,
}

#[non_exhaustive]
#[derive(Debug)]
pub enum LibraryCategory {
    /// Core libraries
    Core,

    /// External libraries
    External,
}

impl fmt::Display for Crate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} @ {:?}", self.crate_type, self.name, self.canonical_path)?;
        if self.dependencies.len() > 1 {
            write!(f, " ({} dependencies)", self.dependencies.len())?;
        } else {
            write!(f, " (1 dependency)")?;
        }

        Ok(())
    }
}

impl fmt::Display for CrateType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Kernel => write!(f, "Kernel"),
            Self::Domain => write!(f, "Domain"),
            Self::Library => write!(f, "Library"),
            _ => write!(f, "Unknown"),
        }
    }
}

impl fmt::Display for Dependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Local(d) => write!(f, "{}", &d),
            Self::Remote(d) => write!(f, "Remote {}", &d.name),
            Self::Illegal(d) => write!(f, "Illegal {}", &d.name),
        }
    }
}

impl fmt::Display for LocalDependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Local {}", &self.name)?;

        if !self.is_resolvable {
            return write!(f, " (broken)");
        }

        Ok(())
    }
}

impl Dependency {
    pub fn name(&self) -> &str {
        match self {
            Self::Local(d) => &d.name,
            Self::Remote(d) => &d.name,
            Self::Illegal(d) => &d.name,
        }
    }
}

impl Crate {
    pub fn print_deps(&self, project: Option<&Project>) {
        for dep in &self.dependencies {
            print!(" \\- {}", dep);
            if let Some(project) = project {
                if let Dependency::Local(d) = dep {
                    if d.is_resolvable {
                        let lookup = project.lookup(&d.name);
                        if let Some(c) = lookup {
                            println!(" -> {}", c);
                            continue;
                        }
                    }
                }
            }
            println!();
        }
    }

    pub fn fix_broken_deps(&self, project: &Project) -> Vec<LocalDependency> {
        let mut r = Vec::new();

        for dep in &self.dependencies {
            if let Dependency::Local(d) = dep {
                if !d.is_resolvable {
                    let lookup = project.lookup(&d.name);
                    if let Some(c) = lookup {
                        // FIXME: Have some kind of factory fn
                        r.push(LocalDependency {
                            name: d.name.clone(),
                            canonical_path: Some(c.canonical_path.clone()),
                            is_resolvable: true,
                        });
                    }
                }
            }
        }

        r
    }

    pub fn read_cargo_toml(&self, project: &Project) -> Result<String, io::Error> {
        let file = project.root.join(&self.canonical_path).join("Cargo.toml");
        let mut file = File::open(file)?;

        let mut r = String::new();
        file.read_to_string(&mut r)?;

        Ok(r)
    }
}

