# BackupMyDevStuff

Zip your dev folder for easy backups !  

## Usage
```
backupmydevstuff [DIR_PATH] [ARCHIVE_PATH]
```


## Features
- uses multiple threads
- respects .gitignore
- project detection when no .gitignore is found
    - NodeJS
        - exclude node_modules/
    - Rust
        - exclude target/

## Build

```
git clone https://github.com/truelossless/BackupMyDevStuff
cd BackupMyDevStuff
cargo build --release
```