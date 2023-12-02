use crate::repoinfo::RepoInfo;
use std::path::Path;
use std::path::PathBuf;
use log::{info, warn};
use tribool::Tribool;
use std::fs;


#[derive(Debug)]
pub struct RepoManagerOptions {
    pub repo_cache_path: PathBuf,
    pub repo_raw_cache_path: PathBuf,
    pub repo_solv_cache_path: PathBuf,
    pub repo_packages_cache_path: PathBuf,
    pub known_repos_path: PathBuf,
    pub known_services_path: PathBuf,
    pub plugins_path: PathBuf,
    pub probe: bool,

    /**
     * Target distro ID to be used when refreshing repo index services.
     * Repositories not maching this ID will be skipped/removed.
     *
     * If empty, \ref Target::targetDistribution() will be used instead.
     */
    pub services_target_distro: String,
}

impl RepoManagerOptions {
    pub fn new<P: AsRef<Path>>(sys_root: P) -> Self {
        let repo_cache_path = sys_root.as_ref().join("var/cache/zypp");
        let config_path = sys_root.as_ref().join("etc/zypp");
        Self {
            repo_raw_cache_path: repo_cache_path.join("raw"),
            repo_solv_cache_path: sys_root.as_ref().join("solv"),
            repo_packages_cache_path: sys_root.as_ref().join("packages"),
            known_repos_path: config_path.join("repos.d"),
            known_services_path: config_path.join("services.d"),
            plugins_path: sys_root.as_ref().join("usr/lib/zypp/plugins"),
            probe: false,
            services_target_distro: Default::default(),
            repo_cache_path: repo_cache_path,
        }
    }
}

#[derive(Debug)]
pub struct RepoManager {
    options: RepoManagerOptions,
    pub repositories: Vec<RepoInfo>,
}

impl RepoManager {

    pub fn new(options: RepoManagerOptions) -> Self {
        info!("Loading known repositories.");
        let mut s = Self {
            options: options,
            repositories: Default::default(),
        };

        if s.options.known_repos_path.exists() {
            let entries = fs::read_dir( &s.options.known_repos_path );
            if !entries.is_ok() {
                warn!("Failed to read directory: {}. {} ", s.options.known_repos_path.as_os_str().to_str().unwrap_or("<empty path>"), entries.unwrap_err() );
            } else {

                let infos = entries.unwrap()
                    // filter errors
                    .filter_map( |e| e.ok() )
                    // filter non files
                    .filter(|e| e.metadata().map_or_else(|_|false, |e| e.is_file() ) )
                    // map dir entry to RepoInfo, filtering errors
                    .map(|e| RepoInfo::read_from_file(e.path()).unwrap_or_else(|e| { warn!( "Failed to read repo file. {}", e); Vec::<RepoInfo>::new() }))
                    // flatten so we can consume it easily
                    .flatten()
                ;

                for mut rInfo in infos {
                    rInfo.raw_gpg_check = Tribool::True;

                    s.repositories.push( rInfo );
                }
            }
        }
        return s;
    }

    pub async fn refreshMetadata( repos: &Vec<RepoInfo> ) {

    }
}
