use std::{
    fs::{self, DirEntry},
    io,
};

use crate::types::ChainsVec;

pub fn read_json_file(file_path: &str) -> Result<ChainsVec, io::Error> {
    // Read the file to a string
    let data = fs::read_to_string(file_path)?;

    // Parse the string into the struct
    let chain: ChainsVec = serde_json::from_str(&data)?;

    Ok(chain)
}

pub fn read_artifacts(path: &str) -> Result<Vec<DirEntry>, io::Error> {
    let artifacts = fs::read_dir(path).unwrap();

    let mut dir_entries = vec![];
    for dir in artifacts.into_iter() {
        dir_entries.push(dir.unwrap());
    }

    Ok(dir_entries)
}
