use std::{
    fs::{self, DirEntry},
    io,
};

use log::info;
use serde::Serialize;
use serde_json::Value;

use super::types::ChainsVec;

pub fn pretty_print(obj: &Value) {
    let mut buf = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
    obj.serialize(&mut ser).unwrap();
    info!("{}", String::from_utf8(buf).unwrap());
}

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
