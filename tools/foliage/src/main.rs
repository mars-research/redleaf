use clap::{Arg, App, AppSettings, SubCommand};
use log::{debug, info, error};
use pretty_env_logger;

mod utils;
mod crates;
mod project;

use crates::Crate;
use project::Project;

fn main() {
    pretty_env_logger::init();

    let app = App::new("Foliage")
                      .version("0.1")
                      .author("The RedLeaf Authors")
                      .about("Helper tool for the RedLeaf project")
                      .setting(AppSettings::SubcommandRequiredElseHelp)
                      .subcommand(SubCommand::with_name("crate")
                                             .about("Get information about a crate")
                                             .arg(Arg::with_name("name")
                                                      .takes_value(true)
                                                      .required(true)
                                                      .index(1)))
                      .subcommand(SubCommand::with_name("fix-deps")
                                             .about("Attempt fix the dependency paths of a crate")
                                             .arg(Arg::with_name("name")
                                                      .takes_value(true)
                                                      .required(true)
                                                      .index(1))
                                             .arg(Arg::with_name("in-place")
                                                      .short("i")
                                                      .help("Edit the Cargo.toml in-place")
                                                      .takes_value(false)))
                      .subcommand(SubCommand::with_name("fix-all-deps")
                                             .about("Attempt fix the dependency paths of all crates in place")
                                             .arg(Arg::with_name("overwrite-my-files")
                                                      .help("I know that it will overwrite my files")
                                                      .required(true)
                                                      .takes_value(false)))
                      .subcommand(SubCommand::with_name("test-populate")
                                             .about("Populate the crate database and exit"));
    let root = utils::find_root().unwrap();
    debug!("Using RedLeaf root {:?}", root);

    let mut project = Project::new(root);

    match project.populate() {
        Ok(c) => {
            info!("Discovered {} crates", c);
        }
        Err(e) => {
            error!("Failed to discover crates: {:?}", e);
            return;
        }
    }

    let matches = app.get_matches();

    if let Some(matches) = matches.subcommand_matches("crate") {
        // Guaranteed to exist by clap
        let name = matches.value_of("name").unwrap();
        match project.lookup(&name) {
            Some(pb) => {
                println!("{}", pb);
                pb.print_deps(Some(&project));
            }
            None => {
                error!("Found nothing :(");
            }
        }

        // let filename = matches.value_of("file").unwrap();
        // println!("{:?}", project.parse_cargo_toml(&PathBuf::from(filename)));
    }

    if let Some(matches) = matches.subcommand_matches("fix-deps") {
        let name = matches.value_of("name").unwrap();
        let c = match project.lookup(&name) {
            Some(c) => c,
            None => {
                error!("No crate named {}", &name);
                return;
            }
        };

        fix_dep_interactive(c, &project, matches.is_present("in-place"));
    }

    if let Some(matches) = matches.subcommand_matches("fix-all-deps") {
        use std::fs::OpenOptions;
        use std::io::Write;

        for name in project.iter_crates() {
            println!("Processing {}", name);
            let c = project.lookup(&name).unwrap();
            fix_dep_interactive(c, &project, true);
        }
    }
}

fn fix_dep_interactive(c: &Crate, p: &Project, inplace: bool) {
    use std::fs::OpenOptions;
    use std::io::Write;

    let new_deps = c.fix_broken_deps(&p);

    let old_toml: String = c.read_cargo_toml(&p).unwrap();
    let new_toml = utils::replace_cargo_toml_deps(old_toml, &c.canonical_path, new_deps);

    if inplace {
        let mut file = OpenOptions::new()
                                   .write(true)
                                   .truncate(true)
                                   .open(c.canonical_path.join("Cargo.toml"))
                                   .expect("Could not open Cargo.toml to write");

        file.write_all(new_toml.as_bytes()).expect("Could not write new Cargo.toml");
    } else {
        print!("{}", &new_toml);
    }
}
