use zypp_rs::solv::pool;

fn main() {

    let mut pool = pool::Pool::new();
    pool.set_rootdir("/").expect("Failed to set Rootdir");
    println!( "Hello, world: {} !", pool.get_rootdir() );
}
