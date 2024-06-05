fn main() -> Result<(), std::io::Error> {
    let mut args = std::env::args_os().skip(1);
    let Some(romfs_path) = args.next() else {
        panic!("Pass a ROMFS file as the first argument");
    };
    let unpack_name = args.next().map(|os| os.into_string().unwrap());
    let data = std::fs::read(romfs_path)?;

    let r = match neotron_romfs::RomFs::new(&data) {
        Ok(r) => r,
        Err(e) => {
            panic!("Not a valid ROMFS image: {:?}", e);
        }
    };

    for entry in &r {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                eprintln!("Error unpacking ROMFS: {:?}", e);
                break;
            }
        };
        let time_str = format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            entry.metadata.ctime.year_since_1970 as u32 + 1970,
            entry.metadata.ctime.zero_indexed_month + 1,
            entry.metadata.ctime.zero_indexed_day + 1,
            entry.metadata.ctime.hours,
            entry.metadata.ctime.minutes,
            entry.metadata.ctime.seconds,
        );
        println!(
            "Found name={:?}, ctime={}, size={}",
            entry.metadata.file_name, time_str, entry.metadata.file_size
        );
        if let Some(unpack_name) = unpack_name.as_deref() {
            if entry.metadata.file_name == unpack_name {
                std::fs::write(unpack_name, entry.contents)?;
            }
        }
    }

    Ok(())
}
