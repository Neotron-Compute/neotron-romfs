## Neotron ROMFS Library

A `no_std` library for creating and parsing ROMFS images.

```rust
fn process_rom(data: &[u8]) -> Result<(), neotron_romfs::Error> {
    let romfs = neotron_romfs::RomFs::new(data)?;
    for entry in romfs {
        if let Ok(entry) = entry {
           println!("{} is {} bytes", entry.metadata.file_name, entry.metadata.file_size);
        }
    }
    Ok(())
}
```

## Licence

Copyright (c) The Neotron Developers, 2024

Licensed under either [MIT](./LICENSE-MIT) or [Apache-2.0](./LICENSE-APACHE) at
your option.
