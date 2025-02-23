# uploader
[![build](https://github.com/paasim/uploads/workflows/build/badge.svg)](https://github.com/paasim/uploads/actions)

File upload service. Does not really do anything but file uploads and downloads.

## install

The [release builds](https://github.com/paasim/uploads/releases) contain a debian package. `man uploads` contains minimal usage documentation.

## development

Dev server can be started with `make run`. It initializes the database, relevant environment variables and starts the server.

For development purposes, [`sqlx-cli`](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md), which is needed for db and query metadata initialization. It can be installed with `cargo binstall sqlx-cli`.
