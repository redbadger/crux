use clap::Parser;

#[derive(Parser)]
#[command(name = "crux", bin_name = "crux")]
pub(crate) enum CruxCli {
    Init,
}
