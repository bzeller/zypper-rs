use ini::Ini;
use tribool::{self, Tribool};
use tribool::Tribool::Indeterminate;
use url::Url;
use std::str::FromStr;
use std::string::ToString;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseRepoFileError {
  #[error(transparent)]
  ParserError(#[from] ini::Error),
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
}

impl RepoInfo {

  fn from_section( sec: &str, prop: &ini::Properties ) -> Result<RepoInfo,Error> {
    let mut info = RepoInfo{ repo_alias: String::from(sec), ..Default::default() };
    for ( key, val ) in prop.iter() {
      match key {
        "type" => info.repo_type = RepoType::from_str(val)?,
        "name" => info.repo_name = val.to_owned(),
        "raw_gpg_check" => info.raw_gpg_check = Tribool::from_str(val).map_err(|_e| ParseRepoFileError::InvalidValue { key: key.to_owned(), value: val.to_owned() } )?,
        "baseurl" => println!("Seen baseurl {}", val),
        &_ => println!("Seen {} {}", key, val), //ignore unknown fields
      }
    }
    Ok(info)
  }

  pub fn read_from_file<P: AsRef<Path>>( file_path: P) -> Result<Vec<RepoInfo>, Error> {
    let mut res: Vec<RepoInfo> = Vec::new();

    let repo_file = Ini::load_from_file(file_path).map_err(ParseRepoFileError::from)?;

    for ( sec, prop ) in repo_file.iter()  {
      if sec.is_none() || sec.unwrap().len() == 0  {
        continue;
      }
      res.push( RepoInfo::from_section( &sec.unwrap(), &prop )? );
    }
    Ok(res)
  }
}
