use zypp_rs::solv::pool;
use zypp_rs::error::{ZyppError};
use zypp_rs::repoinfo::{self, RepoInfo};

fn main() {
    let mut pool = pool::Pool::new();
    pool.set_rootdir("/").expect("Failed to set Rootdir");
    println!( "Hello, world: {} !", pool.get_rootdir() );

    RepoInfo::read_from_file("/etc/zypp/repos.d/anydesk.repo").unwrap();

}
