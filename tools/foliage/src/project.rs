use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Read;

use cargo_toml::{Manifest, Dependency as CargoDependency};

use log::{debug, info, warn, error};

use super::crates::{
    Crate,
    CrateType,
    CrateScope,
    Dependency,
    LocalDependency,
    RemoteDependency,
    IllegalDependency,
};
use super::utils;

pub struct Project {
    pub root: PathBuf,
    crates: HashMap<String, Crate>,
}

impl Project {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            crates: HashMap::new(),
        }
    }

    pub fn populate(&mut self) -> Result<usize, &'static str> {
        let files = utils::find_cargo_tomls(&self.root)?;
        self.populate_by_paths(files)
    }

    pub fn populate_by_paths(&mut self, files: Vec<PathBuf>) -> Result<usize, &'static str> {
        self.crates.clear();

        for file in files {
            match self.parse_cargo_toml(&file) {
                Ok(oc) => {
                    if let Some(c) = oc {
                        info!("Discovered {}", c);
                        if self.crates.contains_key(&c.name) {
                            error!("Naming conflict: Discovered {}, but we already have {}", &c, self.crates.get(&c.name).unwrap());
                        } else {
                            self.crates.insert(c.name.clone(), c);
                        }
                    }
                }
                Err(_) => {
                    // FIXME
                }
            }
        }

        Ok(self.crates.len())
    }

    pub fn lookup<S: AsRef<str>>(&self, name: S) -> Option<&Crate> {
        // Simple right :)
        // TODO: Do path-based "fuzzy" lookup
        self.crates.get(name.as_ref())
    }

    pub fn iter_crates(&self) -> std::collections::hash_map::Keys<'_, String, Crate> {
        self.crates.keys()
    }

    fn parse_cargo_toml(&self, path: &Path) -> Result<Option<Crate>, &'static str> {
        let path = if path.is_absolute() {
            if !path.starts_with(&self.root) {
                error!("Manifest is out of tree: {:?}", path);
                return Err("Manifest is out of tree");
            } else {
                path.to_owned()
            }
        } else {
            self.root.join(path)
        };

        let mut file = File::open(&path).or_else(|e| {
            error!("Could not open file {:?}: {:?}", path, e);
            return Err("Could not open file");
        })?;

        let mut raw = String::new();
        file.read_to_string(&mut raw).or_else(|e| {
            error!("Could not read file {:?}: {:?}", path, e);
            return Err("Could not read file");
        })?;
        
        let manifest = Manifest::from_str(&raw).or_else(|e| {
            error!("Could not process file {:?}: {:?}", path, e);
            Err("Could not process file")
        })?;

        if let None = manifest.package {
            return Ok(None);
        }

        // Normal crate?
        let abs = path.parent().expect("Path has no parent?");
        let canonical_path = abs.strip_prefix(&self.root).expect("Cannot strip prefix?").to_owned();

        let crate_type;
        let crate_scope;
        if canonical_path.starts_with("domains") {
            crate_scope = CrateScope::Domains;
            if canonical_path.starts_with("domains/lib") {
                crate_type = CrateType::Library;
            } else {
                crate_type = CrateType::Domain;
            }
        } else if canonical_path.starts_with("kernel") {
            crate_scope = CrateScope::Kernel;
            if canonical_path.starts_with("kernel/lib") {
                crate_type = CrateType::Library;
            } else {
                crate_type = CrateType::Kernel;
            }
        } else {
            crate_scope = CrateScope::Shared;
            crate_type = CrateType::Library;
        }

        let dependencies: Vec<Dependency> = manifest.dependencies.iter().map(|(name, dep)| {
            match dep {
                CargoDependency::Simple(_) => Dependency::Remote(RemoteDependency {
                    name: name.clone(),
                }),
                CargoDependency::Detailed(d) => {
                    let sources = vec![
                        d.registry.is_some(),
                        d.path.is_some(),
                        d.git.is_some(),
                    ].iter().fold(0, |acc, x| if *x { acc + 1 } else { acc });
                    
                    if sources > 1 {
                        return Dependency::Illegal(IllegalDependency {
                            name: name.clone(),
                        });
                    } else if d.path.is_some() {
                        // Local dep?
                        let reldep = d.path.as_ref().unwrap();
                        let absdep = abs.join(PathBuf::from(reldep));
                        let toml = absdep.join(PathBuf::from("Cargo.toml"));
                        if !absdep.starts_with(&self.root) || !toml.is_file() {
                            return Dependency::Local(LocalDependency {
                                name: name.clone(),
                                is_resolvable: false,
                                canonical_path: None,
                            });
                        } else {
                            let dep_canonical = absdep.strip_prefix(&self.root).expect("Cannot strip prefix?").to_owned();

                            return Dependency::Local(LocalDependency {
                                name: name.clone(),
                                is_resolvable: true,
                                canonical_path: Some(dep_canonical),
                            });
                        }
                    } else {
                        return Dependency::Remote(RemoteDependency {
                            name: name.clone(),
                        });
                    };
                },
            }
        }).collect();

        // FIXME: Factory
        Ok(Some(Crate {
            name: manifest.package.unwrap().name,
            canonical_path,
            crate_type,
            crate_scope,
            dependencies,
        }))
    }
}
