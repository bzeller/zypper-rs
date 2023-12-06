use std::path::{PathBuf, Path};
use std::str::FromStr;

use zypp_rs::solv::pool;
use zypp_rs::error::{ZyppError};
use zypp_rs::repoinfo::RepoInfo;
use zypp_rs::repomanager::{RepoManager, RepoManagerOptions};
use zypp_rs::media::manager::Manager;
use zypp_rs::media::spec::{MediaSpec, FileSpec};
use url::Url;

use tokio::main;

#[tokio::main]
async fn main() {

    let manager = Manager::new();
    let media = manager.attach(&vec![Url::from_str("http://download.opensuse.org").expect("Url should be valid")], &MediaSpec { label: String::from_str("my medium").expect("msg"), medianr: 0, verify_data_path: Default::default() }).await;

    if let Ok(media) = media {
        let res = manager.fetch( &media, Path::new("/history/list").to_owned(),&FileSpec { checkExistsOnly: false, optional: false, ..Default::default() }).await;
        if let Ok(res) = res {
            print!("File was downloaded to: {}", res.as_os_str().to_str().expect("Path was empty") );
        } else {
            print!("File downloading failed horribly: {}", res.unwrap_err() )
        }
    } else {
        println!("Failed to attach {}", media.unwrap_err());
    }


    /*
    let mut pool = pool::Pool::new();
    pool.set_rootdir("/").expect("Failed to set Rootdir");
    println!( "Hello, world: {} !", pool.get_rootdir() );

    let rManager = RepoManager::new( RepoManagerOptions::new("/") );
    for rep in rManager.repositories {
        println!("Found repoinfo {}", rep.repo_alias);
        rep.base_urls.iter().for_each(|u| println!("\tbaseUrl: {}", u))
    }
    */
}
