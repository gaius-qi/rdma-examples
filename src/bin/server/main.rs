fn main() {
    let devices = ibverbs::devices().unwrap();
   println!("Found {} devices", devices.len());
}
