use configparser::ini::Ini;
use tribool::{self, Tribool};
use url::Url;
use std::str::FromStr;
use std::collections::HashMap;
use std::string::ToString;
use std::path::{Path, PathBuf};
use thiserror::Error;
use log::warn;

#[derive(Error, Debug)]
pub enum ParseRepoFileError {
  #[error("Failed to parse file with error: {0}.")]
  ParserError(String),
  #[error("Value {value} for key {key} is not valid")]
  InvalidValue{
    key: String,
    value: String
  }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("The repo type {0} is not known")]
    UnknownRepoType(String),

    #[error(transparent)]
    ParseRepoFileError(#[from] ParseRepoFileError)
}

#[derive(Debug)]
pub enum RepoType {
  None,
  RpmMd,
  Yast2,
  RpmPlainDir,
}

impl Default for RepoType {
    fn default() -> Self {
        Self::None
    }
}

impl FromStr for RepoType {
  type Err = Error;

  #[inline]
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_lowercase().as_str() {
      "rpm-md"|"rpm"|"rpmmd"|"repomd"|"yum"|"up2date"  => Ok(RepoType::RpmMd),
      "yast2"|"yast"|"susetags" => Ok(RepoType::Yast2),
      "plaindir" => Ok(RepoType::RpmPlainDir),
      "NONE"|"none" => Ok(RepoType::None),
      &_ => Err(Error::UnknownRepoType(String::from(s))),
    }
  }
}

impl ToString for RepoType {

    #[inline]
    fn to_string(&self) -> String {
        match self {
          RepoType::None => "none".to_owned(),
          RepoType::RpmMd => "rpm-md".to_owned(),
          RepoType::Yast2 => "yast2".to_owned(),
          RepoType::RpmPlainDir => "NONE".to_owned(),
        }
    }
}
#[derive(Default, Debug)]
pub struct RepoInfo
{
  pub repo_alias: String,
  pub repo_name: String,
  pub repo_type: RepoType,
  pub raw_gpg_check: tribool::Tribool,
  pub base_urls: Vec<Url>,
  metadata_path: PathBuf,
  packages_path: PathBuf
}

impl RepoInfo {

  fn from_section( sec: &str, prop: &HashMap<String, Option<String>> ) -> Result<RepoInfo,Error> {
    let mut info = RepoInfo{ repo_alias: String::from(sec), ..Default::default() };
    for ( key, maybeVal ) in prop.iter() {
      if ( maybeVal.is_none() ) {
        continue;
      }
      
      let val = maybeVal.as_ref().unwrap();
      let values: Vec<&str> = val.split('\n').collect();
      if values.is_empty() {
        continue;
      }

      // convenience access to the first value, usually we only need that
      let first_val: &str = values.first().unwrap();

      println!("Found key: {key} and val: {val}");
      match key.to_lowercase().as_str() {
        "type" => info.repo_type = RepoType::from_str(first_val)?,
        "name" => info.repo_name = first_val.to_owned(),
        "raw_gpg_check" => {
            info.raw_gpg_check = Tribool::from_str(val.as_str()).map_err(|_e| ParseRepoFileError::InvalidValue { key: key.to_owned(), value: first_val.to_owned() } )?
        },
        "baseurl" => {
          for urlstr in values {
            info.base_urls.push( Url::from_str(urlstr).map_err( |e| ParseRepoFileError::InvalidValue { key: key.to_owned(), value: val.to_owned() } )? );
          }
        },
        &_ => warn!("Seen unknown key {} with value {}", key, val), //ignore unknown fields but log them
      }
    }
    Ok(info)
  }

  fn probe_cache<P: AsRef<Path>>( cache_path: P ) -> RepoType {
    let mut rtype = RepoType::None;
    if !cache_path.as_ref().is_dir() {
      if cache_path.as_ref().join("repodata/repomd.xml").is_file() {
        rtype = RepoType::RpmMd;
      }
      else if  cache_path.as_ref().join("content").is_file() {
        rtype = RepoType::Yast2;
      }
      else if cache_path.as_ref().join("cookie").is_file() {
        rtype = RepoType::RpmPlainDir;
      }
    }
    return rtype;
  }

  pub fn read_from_file<P: AsRef<Path>>( file_path: P) -> Result<Vec<RepoInfo>, Error> {
    let mut res: Vec<RepoInfo> = Vec::new();

    let mut config = Ini::new();
    config.set_multiline(true);
    
    let repo_file = config.load(file_path).map_err(ParseRepoFileError::ParserError)?;

    for ( sec, prop ) in repo_file.iter()  {
      if sec.len() == 0  {
        continue;
      }
      res.push( RepoInfo::from_section( &sec, &prop )? );
    }
    Ok(res)
  }

  pub fn set_metadata_path<P: AsRef<Path>>( & mut self, new_path: P ) {
    self.metadata_path = new_path.as_ref().to_path_buf();
  }

  pub fn set_packages_path<P: AsRef<Path>>( & mut self, new_path: P ) {
    self.packages_path = new_path.as_ref().to_path_buf();
  }

  pub fn metadata_path( &self ) -> &PathBuf {
    &self.metadata_path
  }

  pub fn packages_path( &self ) -> &PathBuf {
    &self.packages_path
  }

}
