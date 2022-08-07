pub fn hex(_in: impl AsRef<[u8]>) -> Result<String, std::fmt::Error> {
    let mut s = String::new();
    for byte in _in.as_ref() {
        use std::fmt::Write;
        // println!("{:02x}", byte);
        write!(&mut s, "{:02x}", byte)?;
    }
    Ok(s)
}
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
