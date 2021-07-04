use rusty_vainfo::*;

fn main() {
    let va_inst = VaInstance::new().expect("Failed to grab a VaInstance.");
    let (major, minor) = va_inst.version();
    let vendor_string = va_inst.vendor_string();
    let mut profiles = va_inst.profiles().expect("Failed to fetch profiles.");
    profiles.sort_by_cached_key(|x| x.name.clone());

    println!("VA-API version: {}.{}", major, minor);
    println!("Driver version: {}", vendor_string);

    for profile in profiles {
        println!("    {:<36}:{}", profile.name, profile.entrypoints.join(" | "));
    }
}
