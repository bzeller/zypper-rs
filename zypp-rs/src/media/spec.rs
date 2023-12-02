use std::path::PathBuf;
use byte_unit::Byte;
use tribool::Tribool::{self, True, False, Indeterminate};

#[derive(Debug, Clone)]
pub struct MediaSpec {
    pub label: String,
    pub medianr: u16,
    pub verify_data_path: Option<PathBuf>
}

impl MediaSpec {
    pub fn is_same_medium( &self, other: &MediaSpec ) -> Tribool {

        // first check if we have the same media data
        if self.verify_data_path != other.verify_data_path {
            return false.into();
        }

        // if the verify file is not empty check the medianr
        if self.verify_data_path.is_some() {
            return ( self.medianr == other.medianr ).into();
        }
  
        // can't tell without the URL
        return Indeterminate;
    }
}

#[derive(Debug, Clone)]
pub struct FileSpec {

    pub checkExistsOnly : bool,
    pub optional : bool,
    pub downloadSize : Byte,
    
    //zypp::CheckSum  _checksum;

    pub openSize : Byte,
    //zypp::CheckSum  _openChecksum;

    pub headerSize: Byte,
    //zypp::CheckSum  _headerChecksum;

    pub deltafile: PathBuf
}