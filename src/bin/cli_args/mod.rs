use clap::Parser;
use clap_verbosity_flag;

#[derive(Parser)]
#[command(author = "Peter Grace <pete.grace@gmail.com>")]
#[command(about = "download and commit grafana dashboards")]
pub struct CliArgs {
    #[clap(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity,
}
