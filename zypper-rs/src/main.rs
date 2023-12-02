use std::path::{PathBuf, Path};

use zypp_rs::solv::pool;
use zypp_rs::error::{ZyppError};
use zypp_rs::repoinfo::RepoInfo;
use zypp_rs::repomanager::{RepoManager, RepoManagerOptions};
use zypp_rs::media::manager::Manager;

#[tokio::main]
async fn main() {

    let manager = Manager::new();
    let bMgr = &mut manager;
    let media = manager.attach(!vec!["http://download.opensuse.org"], MediaSpec { label: "my medium", medianr: 0, verify_data_path: Default::default() }).await;

    if let Ok(media) = media {
        let res = bMgr.fetch( &medium, Path::new("/").to_owned(),FileSpec { checkExistsOnly: false, optional: false, ..Default::default() }).await;
        if let Ok(res) = res {
            print!("File was downloaded to: {}", res);
        } else {
            print!("File downloading failed horribly: {}", res.err())
        }
    } else {
        println!("Failed to attach {}", media.err());
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
