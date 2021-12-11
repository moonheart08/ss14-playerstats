use std::str::FromStr;
use warp::Filter;
use reqwest::{Url, Client};
use serde::{Deserialize};
use std::sync::Arc;
use std::time::Duration;

#[derive(Deserialize, Debug)]
struct Response {
    players: usize,
}

#[derive(Deserialize, Debug, Clone)]
struct ServerEntry {
    address: String,
    name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    let total_players_endpoint = warp::path!("total_players")
        .then(|| async move {
            let cli = Arc::new(Client::new());
            if let Ok(servers) = query_servers(cli.clone()).await {
                return generate_prometheus_response(servers, cli).await;
            } else {
                return "".to_string();
            }
        });

    warp::serve(total_players_endpoint)
        .run(([127, 0, 0, 1], 3030))
        .await;
    Ok(())
}

async fn get_total_players(servers: Vec<ServerEntry>, cli: Arc<Client>) -> usize {
    let mut handles = vec![];
    for x in servers {
        let e = x.clone();
        let cli = cli.clone();
        let handle = tokio::spawn( async move {
            if let Ok(url) = Url::from_str(&e.address) {
                return query_player_stats(url, cli).await.unwrap_or(0);
            }
            return 0;
        });
        handles.push(handle);
    }

    let mut sum: usize = 0;

    for x in handles {
        let val = x.await;
        sum += val.unwrap_or(0);
    }

    return sum;

}

async fn query_player_stats(url: Url, cli: Arc<Client>) -> Result<usize, Box<dyn std::error::Error>> {
    if let Some(url) = filter_url(url) {
        let body: Response = cli.get(url).timeout(Duration::new(5, 0)).send().await?.json().await?;
        return Ok(body.players);
    } else {
        return Ok(0); //Server couldn't be got so just pretend it has nobody on
    }
}

async fn query_servers(cli: Arc<Client>) -> Result<Vec<ServerEntry>, Box<reqwest::Error>> {
    return Ok(cli.get("https://central.spacestation14.io/hub/api/servers").send().await?.json().await?);
}

fn filter_url(mut url: Url) -> Option<Url> {
    let mut str: String = url.into();
    str.replace_range(0..=3, "http"); // reqwest pls
    url = Url::from_str(&str).unwrap();

    if url.scheme() == "http" && url.port() == None {
        url.set_port(Some(1212)).unwrap();
    }

    url.set_path(&(url.path().trim_end_matches("/").to_owned() + "/status"));
    return Some(url);
}

async fn generate_prometheus_response(servers: Vec<ServerEntry>, cli: Arc<Client>) -> String {
    return format!("# TYPE ss14_total_player_count gauge\nss14_total_player_count {}", get_total_players(servers, cli).await);
}