use log::{error, info, trace};
use reqwest::Url;
extern crate json;

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

pub fn qbitlist(qbit: &Qbittorrent) -> Vec<Torrent> {
    let mut torrent_list: Vec<Torrent> = Vec::new();

    let mut builder = reqwest::blocking::Client::builder();
    builder = builder.danger_accept_invalid_certs(true);
    builder = builder.cookie_store(true);
    let client = builder.build().unwrap();

    let url = Url::parse(&format!("{}{}", qbit.url, "api/v2/auth/login")).unwrap();
    //  println!("{:?}", url);
    let params = [
        ("username", qbit.username.as_str()),
        ("password", qbit.password.as_str()),
    ];
    let body = serde_urlencoded::to_string(params).unwrap();
    let result = client
        .post(url)
        .header("Referer", &qbit.url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send();

    //  print_type_of(&response);

    match result {
        Ok(r) => {
            if r.status() != 200 {
                error!("Not authenticated: {:?}", r.status());
                return torrent_list;
            } else {
                trace!("Authenticated");
            };
        }
        Err(e) => {
            error!("Error: {:?}", e);
            return torrent_list;
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

    info!("Retrieved list: {}", torrent_list.len());

    torrent_list
}

pub fn qbitdelete(qbit: &Qbittorrent, hash: &str) {
    let mut builder = reqwest::blocking::Client::builder();
    builder = builder.danger_accept_invalid_certs(true);
    builder = builder.cookie_store(true);
    let client = builder.build().unwrap();

    let url = Url::parse(&format!("{}{}", qbit.url, "api/v2/auth/login")).unwrap();
    let params = [
        ("username", qbit.username.as_str()),
        ("password", qbit.password.as_str()),
    ];
    let body = serde_urlencoded::to_string(params).unwrap();
    let result = client
        .post(url)
        .header("Referer", &qbit.url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
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
    let _result = client.get(url).send();

    info!("Torrent removed");
}
