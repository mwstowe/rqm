use log::{debug, error, info, trace, warn};
use reqwest;
use reqwest::StatusCode;
use reqwest::Url;
use serde::{Deserialize, Serialize};
extern crate json;

fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}

pub struct Qbittorrent {
    pub url: String,
    pub username: String,
    pub password: String,
}

#[derive(Clone)]
pub struct Torrent {
    pub pathname: String,
    pub name: String,
    pub hash: String,
    pub category: String,
    pub status: String,
    pub eta: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Auth {
    username: String,
    password: String,
}

pub fn qbitlist(qbit: &Qbittorrent) -> Vec<Torrent> {
    let auth = Auth {
        username: qbit.username.clone(),
        password: qbit.password.clone(),
    };

    let mut torrent_list: Vec<Torrent> = Vec::new();

    let mut builder = reqwest::blocking::Client::builder();
    builder = builder.danger_accept_invalid_certs(true);
    builder = builder.cookie_store(true);
    let client = builder.build().unwrap();

    let url = Url::parse(&format!("{}{}", qbit.url, "api/v2/auth/login")).unwrap();
    //  println!("{:?}", url);
    let result = client
        .post(url)
        .header("Referer", &qbit.url)
        .form(&auth)
        .send();

    //  print_type_of(&response);

    match result {
        Ok(r) => {
            if r.status() != 200 {
                error!("Not authenticated: {:?}", r.status());
                return(torrent_list);
            } else {
                trace!("Authenticated");
            };
        }
        Err(e) => {
            error!("Error: {:?}", e);
            return(torrent_list);
        }
    }

    let url = Url::parse(&format!("{}{}", qbit.url, "api/v2/torrents/info")).unwrap();
    let result = client.get(url).send();

    let r = result.unwrap();
    let body: String = r.text().unwrap();
    let data = json::parse(&body).unwrap();

    //  println!("{:?}", data[0]);

    for i in 0..data.len() {
        torrent_list.push(Torrent {
            pathname: data[i]["save_path"].to_string(),
            name: data[i]["name"].to_string(),
            hash: data[i]["hash"].to_string(),
            category: data[i]["category"].to_string(),
            status: data[i]["state"].to_string(),
            eta: data[i]["eta"].as_u64().unwrap(),
        });
    }

    info!("Retrieved list: {}",torrent_list.len());

    return torrent_list;
}

pub fn qbitdelete(qbit: &Qbittorrent, hash: &str) {
    let auth = Auth {
        username: qbit.username.clone(),
        password: qbit.password.clone(),
    };

    let mut builder = reqwest::blocking::Client::builder();
    builder = builder.danger_accept_invalid_certs(true);
    builder = builder.cookie_store(true);
    let client = builder.build().unwrap();

    let url = Url::parse(&format!("{}{}", qbit.url, "api/v2/auth/login")).unwrap();
    let result = client
        .post(url)
        .header("Referer", &qbit.url)
        .form(&auth)
        .send();

    match result {
        Ok(r) => {
            if r.status() != 200 {
                error!("Not authenticated: {:?}", r.status());
            } else {
                trace!("Authenticated");
            };
        }
        Err(e) => error!("Error: {:?}", e),
    }

    let mut getln: String = "api/v2/torrents/delete?hashes=".to_owned();
    getln.push_str(hash);
    getln.push_str("&deleteFiles=true");

    let url = Url::parse(&format!("{}{}", qbit.url, getln)).unwrap();
    let result = client.get(url).send();

    let r = result.unwrap();

    info!("Torrent removed");

    return;
}
