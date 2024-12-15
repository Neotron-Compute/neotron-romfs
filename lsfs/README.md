# lsfs - a tool for listing the contents of a ROMFS

Make a ROMFS image:

```console
$ cargo run --bin neotron-romfs-mkfs -- ./snake ./flames > rom.fs
   Compiling neotron-romfs-mkfs v0.1.0 (/home/jonathan/Documents/github/neotron-romfs/mkfs)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.23s
     Running `target/debug/neotron-romfs-mkfs ./snake ./flames`
Loading ./snake
Loading ./flames
```

Then you can use this tool to inspect the contents:

```console
$ cargo run --bin neotron-romfs-lsfs -- ./rom.fs
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.01s
     Running `target/debug/neotron-romfs-lsfs ./rom.fs flames`
Found name="snake", ctime=2024-06-05T17:16:51Z, size=1138136
Found name="flames", ctime=2024-06-05T17:16:51Z, size=1090264
```

You can add an extra argument to extract that particular file:

```console
$ cargo run --bin neotron-romfs-lsfs -- ./rom.fs flames
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.01s
     Running `target/debug/neotron-romfs-lsfs ./rom.fs flames`
Found name="snake", ctime=2024-06-05T17:16:51Z, size=1138136
Found name="flames", ctime=2024-06-05T17:16:51Z, size=1090264
$ ls flames
-rw-r--r-- 1 jonathan jonathan 1090264 Jun  5 20:40 flames
```

## Licence

```code
Neotron-ROMFS-lsfs Copyright (c) The Neotron Developers, 2024

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
```

The full text is [here](./GPL-3.0-or-later.txt)
