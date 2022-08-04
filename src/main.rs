mod constants;

use std::fs::{File, read_dir};
use std::io::{BufRead, BufReader, Error, Write};
use std::path::{Path, PathBuf};
use clap::{Parser};
use handlebars::{Handlebars};
use inquire::{validator::StringValidator, Text};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json};
use walkdir::{DirEntry, WalkDir};
use constants::*;

fn kt_file(name: &str) -> String {
    format!("{}{}", name, DOT_KT)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct CommonConfig {
    module: Option<String>,
    base_package_dir: String,
    common_source_set: Option<String>,
    android_source_set: Option<String>,
    ios_source_set: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct AlfredConfig {
    use_koin: Option<bool>,
    common: CommonConfig,
    android: AndroidConfig,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct AndroidConfig {
    module: Option<String>,
    base_package_dir: String,
}

#[derive(Parser, Debug)]
#[clap()]
struct Args {
    #[clap(subcommand)]
    commands: Option<Commands>,

    #[clap(short, long)]
    config: Option<String>,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// runs create command with for a given sub-command. Run `alfred help create` for more info.
    Create(SubcommandCreate),
    /// initializes project with necessary template files. Prints list of newly created files. Run `alfred help init` for more info.
    Init(SubcommandInit),
}

#[derive(clap::Args, Debug)]
struct SubcommandCreate {
    #[clap(subcommand)]
    commands: SubcommandCreateCommands,
}

#[derive(clap::Args, Debug)]
struct SubcommandInit {}

#[derive(clap::Subcommand, Debug)]
enum SubcommandCreateCommands {
    /// creates a common viewmodel
    Viewmodel(ViewmodelSubCommand),
    /// creates an android composable
    Composable(ComposableSubCommand),
    /// creates common viewmodel and android composable
    Feature(FeatureSubCommand),
}

#[derive(clap::Args, Debug)]
struct ViewmodelSubCommand {}

#[derive(clap::Args, Debug)]
struct ComposableSubCommand {}

#[derive(clap::Args, Debug)]
struct FeatureSubCommand {}

fn main() {
    let res = execute();

    match res {
        Ok(_) => std::process::exit(0),
        Err(err) => {
            println!("{}", err.to_string());
            std::process::exit(1)
        }
    }
}

fn execute() -> Result<(), Error> {
    let args = Args::parse();

    let config = parse_config(&args);

    let config = match config {
        Ok(c) => c,
        Err(err) => return Err(err),
    };

    validate_config(&config)?;

    match args.commands {
        Some(command) => match command {
            Commands::Create(create_command) => match create_command.commands {
                SubcommandCreateCommands::Viewmodel(_) => handle_viewmodel(&config)?,
                SubcommandCreateCommands::Composable(_) => handle_composable(&config)?,
                SubcommandCreateCommands::Feature(_) => {
                    handle_viewmodel(&config)?;
                    handle_composable(&config)?;
                }
            },
            Commands::Init(_init_command) => {
                add_missing_viewmodel_classes(&config)?;
                println!("init successful");
            }
        },
        None => {
            // nothing to do here
        }
    }

    Ok(())
}

fn parse_config(args: &Args) -> Result<AlfredConfig, Error> {
    let cwd_path = std::env::current_dir()?;

    // path either comes from config or CLI default
    let config_path_str = match args.config.as_ref() {
        Some(config) => config.as_str(),
        None => CFG_YAML,
    };

    let mut buf = PathBuf::new();

    if config_path_str.starts_with("/") {
        buf.push(config_path_str)
    } else {
        buf.push(&cwd_path);
        buf.push(&config_path_str)
    }

    let full_path = buf.to_str().unwrap();

    match File::open(buf.to_str().unwrap()) {
        Ok(file) => {
            let parsed: serde_yaml::Result<AlfredConfig> = serde_yaml::from_reader(file);
            match parsed {
                Ok(parsed) => Ok(parsed),
                Err(_) => Err(Error::new(
                    std::io::ErrorKind::InvalidInput,
                    String::from(format!("could not parse file {}", config_path_str)),
                )),
            }
        }
        Err(_) => Err(Error::new(
            std::io::ErrorKind::InvalidInput,
            String::from(format!("could not find file {}", full_path)),
        )),
    }
}

fn validate_config(_config: &AlfredConfig) -> Result<(), Error> {
    Ok(())
}

fn render_template_or_err<T>(
    generator: &Handlebars,
    content: &str,
    data: &T,
) -> Result<String, Error>
    where T: Serialize,
{
    let template = generator.render_template(
        content,
        data,
    );

    return match template {
        Ok(t) => Ok(t),
        Err(e) => {
            return Err(Error::new(std::io::ErrorKind::InvalidData, String::from(e.desc)));
        }
    };
}

fn handle_viewmodel(config: &AlfredConfig) -> Result<(), Error> {
    let mut root_package_path = common_package_dir(config)?;
    let package =
        prompt_package_or_err(format!("ViewModel package (relative to {})", root_package_path.to_str().unwrap()).as_str())?;
    let class_name = prompt_class_name("ViewModel class name")?;

    let package_as_dir_path = str::replace(&package, ".", "/");

    root_package_path.push(package_as_dir_path);

    add_missing_viewmodel_classes(config)?;

    let generator = Handlebars::new();
    let vm_string = include_str!("./templates/ViewStateViewModel.mustache");

    let common_dir = common_package_dir(config)?;

    let common_flat = flatten_dir(common_dir.as_path());

    let uistate_path = find_kt_file(&common_flat, UISTATE);
    let atomicjob_path = find_kt_file(&common_flat, ATOMICJOB);

    let uistate_package = find_package_name(&uistate_path).unwrap();
    let atomicjob_package = find_package_name(&atomicjob_path).unwrap();

    let base_package = str::replace(&config.common.base_package_dir, "/", ".");

    let full_package = format!("{}.{}", base_package, package);

    let vm_template = render_template_or_err(
        &generator,
        vm_string,
        &json!({
            NAME: class_name,
            PACKAGE: full_package,
            UISTATE_PACKAGE: uistate_package,
            ATOMICJOB_PACKAGE: atomicjob_package
        }),
    )?;

    create_file(root_package_path.to_str().unwrap(), class_name.as_str(), vm_template)?;

    Ok(())
}

fn handle_composable(config: &AlfredConfig) -> Result<(), Error> {
    let android_base = android_dir(config, DEFAULT_ANDROID_MODULE)?;
    let package =
        prompt_package_or_err(format!("Composable package (relative to {})", android_base.to_str().unwrap()).as_str())?;
    let class_name = prompt_class_name("Composable class name")?;

    add_missing_viewmodel_classes(config)?;

    let handlebars = Handlebars::new();

    let mut android_base_path = android_dir(config, DEFAULT_ANDROID_MODULE)?;
    let package_as_dir = str::replace(package.as_str(), ".", "/").to_owned();
    android_base_path.push(package_as_dir);

    let common_dir = common_package_dir(config)?;

    let common_flat = flatten_dir(common_dir.as_path());

    let uistate_path = find_kt_file(&common_flat, UISTATE);
    let viewstate_viewmodel_path = find_kt_file(&common_flat, VIEWSTATEVIEWMODEL);

    let viewstate_package = find_package_name(&viewstate_viewmodel_path).unwrap();
    let uistate_package = find_package_name(&uistate_path).unwrap();

    let mut full_package = PathBuf::new();
    full_package.push(&config.android.base_package_dir);
    let package_as_dir = str::replace(package.as_str(), ".", "/").to_owned();
    full_package.push(package_as_dir);

    let full_package = str::replace(full_package.to_str().unwrap(), "/", ".");

    let viewstate_package = format!("{}.{}", viewstate_package, VIEWSTATEVIEWMODEL);
    let uistate_package = format!("{}.{}", uistate_package, UISTATE);

    let template = render_template_or_err(
        &handlebars,
        include_str!("./templates/Compose.mustache"),
        &json!({
            NAME: class_name,
            UISTATE_PACKAGE: uistate_package,
            VIEWSTATE_VIEWMODEL_PACKAGE: viewstate_package,
            PACKAGE: full_package
        }),
    )?;

    create_file(android_base_path.to_str().unwrap(), class_name.as_str(), template)?;

    Ok(())
}

fn find_kt_file(dir: &Vec<DirEntry>, name: &str) -> PathBuf {
    dir.iter()
        .find(|entry| {
            match entry.file_name().to_str() {
                Some(str) => str == kt_file(name),
                None => false
            }
        }).unwrap().path().to_owned()
}

fn find_kt_file_optional<'a>(dir: &'a Vec<DirEntry>, name: &str) -> Option<&'a DirEntry> {
    dir.iter()
        .find(|entry| {
            match entry.file_name().to_str() {
                Some(str) => str == kt_file(name),
                None => false
            }
        })
}

fn find_package_name(path: &PathBuf) -> Option<String> {
    let viewstate_reader = BufReader::new(File::open(path).unwrap());

    for (_, line) in viewstate_reader.lines().enumerate() {
        match line {
            Ok(line) => {
                let line_str = line.as_str().to_owned();
                if line_str.starts_with("package ") {
                    return Some(String::from(line_str.strip_prefix("package ").unwrap()));
                }
            }
            Err(_) => {}
        }
    }

    None
}

fn prompt_package_or_err(prompt_message: &str) -> Result<String, Error> {
    let regex = Regex::new(r"(?:^\w+|\w+\.\w+)+$").unwrap();

    let package_validator: StringValidator = &|input| {
        if regex.is_match(input.chars().as_str()) {
            Ok(())
        } else {
            Err(String::from("invalid package syntax"))
        }
    };

    let package = Text::new(prompt_message)
        .with_validator(package_validator)
        .prompt()
        .expect(&"package prompt failed");

    Ok(package)
}

fn prompt_class_name(prompt_message: &str) -> Result<String, Error> {
    let regex = Regex::new(r"^[A-Z]+[a-zA-Z0-9]*$").unwrap();
    let class_name_validator: StringValidator = &|input| {
        if regex.is_match(input.chars().as_str()) {
            Ok(())
        } else {
            Err(String::from("invalid class name syntax"))
        }
    };

    let class_name = Text::new(prompt_message)
        .with_validator(class_name_validator)
        .prompt()
        .expect(&"class name prompt failed");

    Ok(class_name)
}

fn add_missing_viewmodel_classes(config: &AlfredConfig) -> Result<(), Error> {
    let common_main_root_path = common_package_dir(config)?;
    let common_main_root_str = common_main_root_path.to_str().unwrap();

    let android_main_path = common_android_package_dir(config)?;
    let android_main_str = android_main_path.to_str().unwrap();

    let ios_main_path = common_ios_package_dir(config)?;
    let ios_main_str = ios_main_path.to_str().unwrap();

    let common_flat = flatten_dir(common_main_dir(config, DEFAULT_COMMON_SOURCE_SET)?.as_path());
    let android_flat = flatten_dir(android_main_path.as_path());
    let ios_flat = flatten_dir(ios_main_path.as_path());

    let package = str::replace(&config.common.base_package_dir, "/", ".");

    let handlebars = Handlebars::new();

    let viewstate_viewmodel_interfaces = if config.use_koin.unwrap_or(false) {
        ", KoinComponent"
    } else {
        ""
    };

    let common_viewmodel = find_kt_file_optional(&common_flat, VIEWSTATEVIEWMODEL);

    match common_viewmodel {
        None => {
            let template = render_template_or_err(
                &handlebars,
                include_str!("./templates/AbstractViewStateViewModel.mustache"),
                &json!({
                        PACKAGE: package,
                        INTERFACES: viewstate_viewmodel_interfaces
                }),
            )?;
            create_file(common_main_root_str, VIEWSTATEVIEWMODEL, template)?;
        }
        Some(_) => {
            // nothing to do here
        }
    }

    let uistate = find_kt_file_optional(&common_flat, UISTATE);

    match uistate {
        None => {
            let template = render_template_or_err(
                &handlebars,
                include_str!("./templates/UiState.mustache"),
                &json!({
                        PACKAGE: package
                }),
            )?;
            create_file(common_main_root_str, UISTATE, template)?;
        }
        Some(_) => {
            // nothing to do here
        }
    }

    let atomicjob = find_kt_file_optional(&common_flat, ATOMICJOB);

    match atomicjob {
        None => {
            let template = render_template_or_err(
                &handlebars,
                include_str!("./templates/AtomicJob.mustache"),
                &json!({
                        PACKAGE: package
                }),
            )?;
            create_file(common_main_root_str, ATOMICJOB, template)?;
        }
        Some(_) => {
            // nothing to do here
        }
    }

    let android_viewmodel = find_kt_file_optional(&android_flat, PLATFORMVIEWMODEL);

    match android_viewmodel {
        None => {
            let template = render_template_or_err(
                &handlebars,
                include_str!("./templates/AndroidPlatformViewModel.mustache"),
                &json!({
                        PACKAGE: package
                }),
            )?;
            create_file(android_main_str, PLATFORMVIEWMODEL, template)?;
        }
        Some(_) => {
            // nothing to do here
        }
    }

    let ios_viewmodel = find_kt_file_optional(&ios_flat, PLATFORMVIEWMODEL);

    match ios_viewmodel {
        None => {
            let template = render_template_or_err(
                &handlebars,
                include_str!("./templates/IosPlatformViewModel.mustache"),
                &json!({
                        PACKAGE: package
                }),
            )?;
            create_file(ios_main_str, PLATFORMVIEWMODEL, template)?;
        }
        Some(_) => {
            // nothing to do here
        }
    }

    Ok(())
}

fn flatten_dir(path: &Path) -> Vec<DirEntry> {
    let mut paths = Vec::new();
    for entry in WalkDir::new(path) {
        match entry {
            Ok(entry) => {
                paths.push(entry);
            }
            Err(_) => {}
        }
    }

    paths
}

fn create_file(root: &str, name: &str, content: String) -> Result<(), Error> {
    let mut path_buf = PathBuf::new();
    path_buf.push(root);
    let name = String::from(name);
    let _name = kt_file(name.as_str());
    path_buf.push(_name);
    println!("creating file {}", path_buf.to_str().unwrap());
    let prefix = path_buf.parent().unwrap();
    std::fs::create_dir_all(prefix)?;
    let mut new_file = File::create(path_buf)?;
    new_file.write(content.as_bytes())?;
    Ok(())
}

fn android_dir(config: &AlfredConfig, default: &str) -> Result<PathBuf, Error> {
    let cwd_path = std::env::current_dir()?.to_str().unwrap().to_owned();

    let module = config.android.module.to_owned().unwrap_or_else(|| String::from(default));

    let mut dir = PathBuf::new();
    dir.push(cwd_path);
    dir.push(module);
    dir.push(SRC);
    dir.push(MAIN);

    let _dir = read_dir(dir.clone());

    if folder_exists(&dir, KOTLIN) {
        dir.push(KOTLIN)
    } else if folder_exists(&dir, JAVA) {
        dir.push(JAVA)
    }

    dir.push(&config.android.base_package_dir);

    Ok(dir)
}

fn folder_exists(base: &PathBuf, name: &str) -> bool {
    let mut buf = base.clone();
    buf.push(name);
    Path::new(base.as_path()).exists()
}

/// returns full path to common main src dir
fn common_main_dir(config: &AlfredConfig, default: &str) -> Result<Box<PathBuf>, Error> {
    let cwd_path = std::env::current_dir()?.to_str().unwrap().to_owned();

    let source_set = config.common.common_source_set.to_owned().unwrap_or_else(|| String::from(default));
    let module = config.common.module.to_owned().unwrap_or_else(|| String::from(DEFAULT_COMMON_MODULE));

    let mut dir = PathBuf::new();
    dir.push(cwd_path);
    dir.push(module);
    dir.push(SRC);
    dir.push(source_set);

    Ok(Box::new(dir))
}

/// returns full path to common source set package directory
fn common_package_dir(config: &AlfredConfig) -> Result<Box<PathBuf>, Error> {
    source_set_package_dir(config, &config.common.common_source_set, DEFAULT_COMMON_SOURCE_SET)
}

/// returns full path to common android source set package directory
fn common_android_package_dir(config: &AlfredConfig) -> Result<Box<PathBuf>, Error> {
    source_set_package_dir(config, &config.common.android_source_set, DEFAULT_ANDROID_SOURCE_SET)
}

/// returns full path to common ios source set package directory
fn common_ios_package_dir(config: &AlfredConfig) -> Result<Box<PathBuf>, Error> {
    source_set_package_dir(config, &config.common.ios_source_set, DEFAULT_IOS_SOURCE_SET)
}

/// returns full path to source set package directory
fn source_set_package_dir(config: &AlfredConfig, source_set: &Option<String>, default: &str) -> Result<Box<PathBuf>, Error> {
    let cwd_path = std::env::current_dir()?.to_str().unwrap().to_owned();

    let source_set = source_set.to_owned().unwrap_or_else(|| String::from(default));
    let module = config.common.module.to_owned().unwrap_or_else(|| String::from(DEFAULT_COMMON_MODULE));

    let mut dir = PathBuf::new();
    dir.push(cwd_path);
    dir.push(module);
    dir.push(SRC);
    dir.push(source_set);
    dir.push(KOTLIN);
    dir.push(&config.common.base_package_dir);

    Ok(Box::new(dir))
}
