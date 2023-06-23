use clap::Parser;
use configparser::ini::Ini;
// use std::error::Error;
use chrono;
use log::{debug, error, info, trace, warn};
use magic_crypt::{new_magic_crypt, MagicCryptTrait};
use std::process::Command;
use std::time::Duration;
mod qbitapi;

/// rqm - remote qbittorrent manager
#[derive(Parser, Debug)]
#[clap(version, about, long_about = None, setting = clap::AppSettings::DeriveDisplayOrder)]
struct Args {
    /// Config file
    #[clap(short, long)]
    config: Option<String>,
    /// Encrypt password for conf file
    #[clap(short, long)]
    password: Option<String>,
}

#[derive(Debug, Clone)]
struct TorrentCategory {
    category_name: String,
    localpath: String,
    run_script: String,
    notify_script: String,
}

fn main() {
    let mut args = Args::parse();

    if !(args.password.is_none()) {
        let mc = new_magic_crypt!("rqmRQMrqm", 256);

        let base64 = mc.encrypt_str_to_base64(args.password.unwrap());
        println!("Encrypted Password: [{}]", base64);
        return;
    }

    args.config.get_or_insert(String::from("rqm.conf"));

    let mut config = Ini::new();
    info!("Config: {}", args.config.as_ref().unwrap());

    let map = config.load(args.config.as_ref().unwrap());

    trace!("{:?}", map);

    let logfile = config.get("global", "logfile").unwrap();
    let loglevel = config
        .get("global", "loglevel")
        .unwrap_or(String::from("error"));
    setup_logger(logfile, &loglevel);

    let pw = config.get("qbittorrent", "password").unwrap();
    let mc = new_magic_crypt!("rqmRQMrqm", 256);
    let unencrypted_pass = mc.decrypt_base64_to_string(&pw).unwrap();

    let qbit = qbitapi::Qbittorrent {
        url: config
            .get("qbittorrent", "url")
            .unwrap_or(String::from("http://localhost:8080")),
        username: config
            .get("qbittorrent", "username")
            .unwrap_or(String::from("admin")),
        password: unencrypted_pass,
    };

    let check_interval: u64 = config
        .get("qbittorrent", "check_interval")
        .unwrap_or("3600".to_string())
        .parse::<u64>()
        .unwrap();

    let mut categories: Vec<TorrentCategory> = Vec::new();

    let catlist = config
        .get("post processing", "categories")
        .unwrap_or("".to_string());

    let catlist = catlist.split(',').map(|s| s.trim());
    let catlist: Vec<String> = catlist.map(|s| s.to_string()).collect();

    let default_localpath = config.get("post processing", "localpath").unwrap();
    let default_run_script = config
        .get("post processing", "run_script")
        .unwrap_or("".to_string());
    let default_notify_script = config
        .get("post processing", "notify_script")
        .unwrap_or("".to_string());

    categories.push(TorrentCategory {
        category_name: "default".to_string(),
        localpath: default_localpath.clone(),
        run_script: default_run_script.clone(),
        notify_script: default_notify_script.clone(),
    });

    for i in 0..catlist.len() {
        categories.push(TorrentCategory {
            category_name: catlist[i].clone(),
            localpath: config
                .get(&catlist[i], "localpath")
                .unwrap_or(default_localpath.clone()),
            run_script: config
                .get(&catlist[i], "run_script")
                .unwrap_or(default_run_script.clone()),
            notify_script: config
                .get(&catlist[i], "notify_script")
                .unwrap_or(default_run_script.clone()),
        });
    }

    //
    // Main Loop
    //
    loop {
        let mut sleep_interval: u64 = check_interval;
        let torrent_list: Vec<qbitapi::Torrent> = qbitapi::qbitlist(&qbit);

        for torrent in torrent_list {
            if torrent.status == "pausedUP" {
                info!("{}:{} complete", torrent.name, torrent.category);
                loop {
                    sleep_interval = 60;
                    let rsync_cmd: String = config
                        .get("post processing", "rsync")
                        .unwrap_or("rsync".to_string());
                    let mut remote_path = config
                        .get("post processing", "remote_user")
                        .unwrap_or("".to_string());
                    if remote_path.ne("") {
                        remote_path.push_str("@");
                    }
                    remote_path.push_str(&config.get("post processing", "server").unwrap());
                    remote_path.push_str(":");
                    remote_path.push_str(&torrent.pathname);
                    remote_path.push_str(&torrent.name);
                    let local_path = config.get("post processing", "partialpath").unwrap();
                    let output = Command::new(rsync_cmd)
                        .arg("-a")
                        .arg("--partial-dir=.rqm")
                        .arg(remote_path)
                        .arg(&local_path)
                        .output()
                        .expect("rsync failed to start");

                    if output.status.success() {
                        info!("{} transferred", torrent.name);

                        let mut local_full_path = local_path;
                        if !local_full_path.ends_with("/") {
                            local_full_path.push_str("/")
                        }
                        local_full_path.push_str(&torrent.name);

                        let mut run_script = default_run_script.clone();
                        for i in 0..categories.len() {
                            if categories[i].category_name == torrent.category {
                                run_script = categories[i].run_script.clone();
                            }
                        }
                        if run_script.ne("") {
                            info!("Running script: {}", run_script);
                            let scriptout = Command::new(run_script)
                                .arg(&local_full_path)
                                .output()
                                .expect("script failed");
                        }

                        let set_owner = config
                            .get("post processing", "set_owner")
                            .unwrap_or("".to_string());
                        let set_group = config
                            .get("post processing", "set_group")
                            .unwrap_or("".to_string());
                        if set_owner.ne("") {
                            let mut chowner = set_owner;
                            if set_group.ne("") {
                                chowner.push_str(":");
                                chowner.push_str(&set_group);
                            }
                            info!("Changing ownership to {}", chowner);
                            let chownout = Command::new("chown")
                                .arg("-R")
                                .arg(chowner)
                                .arg(&local_full_path)
                                .output()
                                .expect("chown failed");
                        }
                        let mut localdest = default_localpath.clone();
                        for i in 0..categories.len() {
                            if categories[i].category_name == torrent.category {
                                localdest = categories[i].localpath.clone();
                            }
                        }
                        info!("Moving to {}", localdest);
                        let moveout = Command::new("mv")
                            .arg(local_full_path)
                            .arg(&localdest)
                            .output()
                            .expect("move failed");

                        let mut finalpath = localdest;
                        if !finalpath.ends_with("/") {
                            finalpath.push_str("/")
                        }
                        finalpath.push_str(&torrent.name);

                        let mut notify_script = default_notify_script.clone();
                        for i in 0..categories.len() {
                            if categories[i].category_name == torrent.category {
                                notify_script = categories[i].notify_script.clone();
                            }
                        }
                        if notify_script.ne("") {
                            info!("Running script: {}", notify_script);
                            let scriptout = Command::new(notify_script)
                                .arg(&finalpath)
                                .output()
                                .expect("script failed");
                        }

                        qbitapi::qbitdelete(&qbit, &torrent.hash);
                        break;
                    } else {
                        error!(
                            "Rsync failed with status: {}.  Retrying in {}",
                            output.status, sleep_interval
                        );
                        std::thread::sleep(Duration::from_secs(sleep_interval));
                        sleep_interval = sleep_interval * 2;
                    }
                }
            }
            if torrent.eta < sleep_interval {
                sleep_interval = torrent.eta + 120;
            }
        }
        trace!("Sleeping for: {:?}", sleep_interval);
        std::thread::sleep(Duration::from_secs(sleep_interval));
    }
}

fn setup_logger(logfilename: String, loglevel: &str) -> Result<(), fern::InitError> {
    let mut filter_level = log::LevelFilter::Debug;

    match loglevel {
        "debug" => filter_level = log::LevelFilter::Debug,
        "error" => filter_level = log::LevelFilter::Error,
        "info" => filter_level = log::LevelFilter::Info,
        "trace" => filter_level = log::LevelFilter::Trace,
        "warn" => filter_level = log::LevelFilter::Warn,
        _ => filter_level = log::LevelFilter::Info,
    };

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(filter_level)
        .chain(std::io::stdout())
        .chain(fern::log_file(logfilename)?)
        .apply()?;
    Ok(())
}
