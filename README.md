# neotron-romfs

A ROM filing-system for Neotron OS

## What is a ROM filing system?

It's like any other kind of filing system, except the contents are designed to
live in Flash memory next to your microcontroller's firmware.

## What's in this repo?

* [`neotron-romfs`](./neotron-romfs/) - a `no_std` library for encoding and
  decoding ROM FS images
* [`mkfs`](./mkfs/) - a CLI application for packing files into a ROM FS image
* [`lsfs`](./lsfs/) - a CLI application for unpacking files from a ROM FS image

## What's the format?

A ROM FS contains a header, and then a number of entries - all concatenated
together.

There are no CRCs because it is assumed that the Flash storage is reliable. If
this is an issue, feel free to wrap the whole ROM FS with a CRC.

### ROM FS Header

1. The 8-byte ASCII string `NeoROMFS`.
2. The version, as a 4-byte sequence
   1. v1.0.0 is given as `[0x00, 0x01, 0x00, 0x00]`
   2. Other values are reserved for future use
3. The ROM FS size in bytes (including this header), as a 4-byte big-endian
   `u32`

The header is always 24 bytes long.

### ROM FS Entry

An entry comprises a header, followed by the contents of the file.

The header is:

1. A filename, as a null-padded UTF-8 string which is exactly 14 bytes in
   length.
2. A file size, as a 4-byte big-endian `u32` (not including this header)
3. A timestamp, as a six byte value:
    1. The years since 1970 (e.g. `2003` is `33`)
    2. The month, zero-indexed (e.g. February is `1`)
    3. The day of the month, zero-indexed (e.g. the 3rd is `2`)
    4. The hour of the day
    5. The minutes past the hour
    6. The seconds past the minute

## Licence

The file format is licensed as
[CC0](https://creativecommons.org/public-domain/cc0/).

The libraries and binaries each have a licence specified - see their package
files for details.
