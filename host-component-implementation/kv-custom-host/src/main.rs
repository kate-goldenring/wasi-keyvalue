mod echo;
mod table;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[clap(name = "add-host", version = env!("CARGO_PKG_VERSION"))]
struct AddApp {
    /// The string
    s: String,
    /// The path to the component.
    #[clap(value_name = "COMPONENT_PATH")]
    component: PathBuf,
}

impl AddApp {
    async fn run(self) -> anyhow::Result<()> {
        let res = echo::echo(self.component, self.s.clone()).await?;
        println!("{} = {res}", self.s);
        Ok(())
    }
}

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    AddApp::parse().run().await
}
