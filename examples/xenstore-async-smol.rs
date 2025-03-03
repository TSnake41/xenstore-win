use clap::{Parser, Subcommand};
use futures::StreamExt;
use smol::Executor;
use xenstore_rs::{AsyncWatch, AsyncXs};
use xenstore_win::smol::XsSmolWindows;

/// Demo/test tool for xenstore Rust bindings
#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List Xenstore keys in path
    List {
        #[arg()]
        path: String,
    },
    /// Read value of Xenstore path
    Read {
        #[arg()]
        path: String,
    },
    /// Remove value of Xenstore path
    Rm {
        #[arg()]
        path: String,
    },
    /// Write value to Xenstore path
    Write {
        #[arg()]
        path: String,
        #[arg()]
        data: String,
    },
    /// Watch on path on Xenstore.
    Watch {
        #[arg()]
        path: String,
    },
}

fn main() {
    let cli = Cli::parse();

    let executor = Executor::new();

    smol::block_on(executor.run(async {
        let mut xs = XsSmolWindows::new().await.expect("xenstore should open");

        match cli.command {
            Command::List { path } => cmd_list(&mut xs, &path).await,
            Command::Read { path } => cmd_read(&mut xs, &path).await,
            Command::Rm { path } => cmd_rm(&mut xs, &path).await,
            Command::Write { path, data } => cmd_write(&mut xs, &path, &data).await,
            Command::Watch { path } => cmd_watch(&mut xs, &path).await,
        }
    }));

    drop(executor);
}

async fn cmd_list(xs: &mut impl AsyncXs, path: &String) {
    let values = xs.directory(&path).await.expect("path should be readable");
    for value in values {
        println!("{}", value);
    }
}

async fn cmd_read(xs: &mut impl AsyncXs, path: &String) {
    let value = xs.read(&path).await.expect("path should be readable");
    println!("{}", value);
}

async fn cmd_rm(xs: &mut impl AsyncXs, path: &String) {
    xs.rm(&path).await.expect("cannot rm xenstore path");
}

async fn cmd_write(xs: &mut impl AsyncXs, path: &String, data: &String) {
    xs.write(&path, &data)
        .await
        .expect("cannot write to xenstore path");
}

async fn cmd_watch<XS: AsyncXs + AsyncWatch>(xs: &mut XS, path: &String) {
    let mut stream = xs.watch(&path).await.expect("path should be watchable");

    while let Some(entry) = stream.next().await {
        println!("{entry}: {:?}", xs.read(&entry).await);
    }
}
