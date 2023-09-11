use zypp_rs::solv::pool;
use zypp_rs::error::{ZyppError};
use zypp_rs::repoinfo::RepoInfo;
use zypp_rs::repomanager::{RepoManager, RepoManagerOptions};

fn main() {
    let mut pool = pool::Pool::new();
    pool.set_rootdir("/").expect("Failed to set Rootdir");
    println!( "Hello, world: {} !", pool.get_rootdir() );

    let rManager = RepoManager::new( RepoManagerOptions::new("/") );
    for rep in rManager.repositories {
        println!("Found repoinfo {}", rep.repo_alias);
        rep.base_urls.iter().for_each(|u| println!("\tbaseUrl: {}", u))
    }

}
