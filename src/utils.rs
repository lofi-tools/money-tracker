use serde::{de, Deserialize, Deserializer};
// use std::str::FromStr;

pub fn hex(_in: impl AsRef<[u8]>) -> Result<String, std::fmt::Error> {
    let mut s = String::new();
    for byte in _in.as_ref() {
        use std::fmt::Write;
        // println!("{:02x}", byte);
        write!(&mut s, "{:02x}", byte)?;
    }
    Ok(s)
}

pub fn de_from_str<'de, D, Out>(deserializer: D) -> Result<Out, D::Error>
where
    D: Deserializer<'de>,
    Out: std::str::FromStr,
    Out::Err: std::fmt::Display,
{
    let s = String::deserialize(deserializer)?;
    Out::from_str(&s).map_err(de::Error::custom)
}

mod deprecated {
    // trait HexDisplayExt {
    //     fn hex(&self) -> Result<String, std::fmt::Error>;
    // }
    // impl<T: ?Sized + AsRef<[u8]>> HexDisplayExt for T {
    //     fn hex(&self) -> Result<String, std::fmt::Error> {
    //         let mut s = String::new();
    //         for byte in self.as_ref() {
    //             use std::fmt::Write;
    //             write!(&mut s, "{:x}", byte)?;
    //         }
    //         Ok(s)
    //     }
    // }
}
