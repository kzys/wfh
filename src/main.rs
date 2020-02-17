extern crate ignore;
extern crate notify;

#[macro_use]
extern crate log;

#[macro_use]
extern crate structopt;

mod app;
mod term;

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "wfh", about = "synchronize files as you edit")]
struct Options {
    host: String,
    dirs: Vec<String>,
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let options = Options::from_args();
    debug!("{:#?}", options);

    let app = app::App::new(options.host, options.dirs)?;
    app.run()
}
