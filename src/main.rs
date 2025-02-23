mod config;
mod file;
mod index;
mod server;

fn main() -> std::io::Result<()> {
    server::run(config::Config::read_from_env())
}
