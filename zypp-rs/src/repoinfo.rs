use tribool;
use std::str::FromStr;
use std::string::ToString;

enum RepoType {
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
  type Err = ();

  #[inline]
  fn from_str(s: &str) -> Result<RepoType, ()> {
    match s.to_lowercase().as_str() {
      "rpm-md" | "rpm"|"rpmmd"|"repomd"|"yum"|"up2date"  => Ok(RepoType::RpmMd),
      "yast2"	| "yast"|"susetags" => Ok(RepoType::Yast2),
      "plaindir"  => Ok(RepoType::RpmPlainDir),
      "NONE" | "none" => Ok(RepoType::None),
      _      => Err(()),
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

pub struct RepoInfo
{
  repo_type: RepoType,
  raw_gpg_check: tribool::Tribool,

}

impl RepoInfo {

}