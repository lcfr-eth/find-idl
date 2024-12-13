use solana_cli::{
    cli::{process_command, CliCommand, CliConfig},
    program::ProgramCliCommand,
};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair},
};
use std::{env, fs, fs::File, io::Read, path::PathBuf, str::FromStr};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <programId>", args[0]);
        std::process::exit(1);
    }

    let program_pubkey = match Pubkey::from_str(&args[1]) {
        Ok(pubkey) => pubkey,
        Err(_) => {
            eprintln!("Invalid programId provided.");
            std::process::exit(1);
        }
    };

    let keypair = Keypair::new();

    let mut out_file = {
        let current_exe = env::current_exe().unwrap();
        PathBuf::from(current_exe.parent().unwrap().parent().unwrap())
    };
    out_file.set_file_name(format!("{}_program_dump.so", program_pubkey));

    let mut config = CliConfig::default();
    config.signers = vec![&keypair];
    config.command = CliCommand::Program(ProgramCliCommand::Dump {
        account_pubkey: Some(program_pubkey),
        output_location: out_file.clone().into_os_string().into_string().unwrap(),
    });

    match process_command(&config) {
        Ok(_) => {
            println!("Program successfully dumped to {:?}", out_file);

            let mut file = File::open(&out_file).unwrap();
            let mut out_data = Vec::new();
            file.read_to_end(&mut out_data).unwrap();

            println!("Dumped program data size: {}", out_data.len());

            let contains_anchor_idl = out_data
                .windows("anchor:idl".len())
                .any(|window| window == b"anchor:idl");
            if contains_anchor_idl {
                println!("Found 'anchor:idl' in the program data.");
            } else {
                println!("'anchor:idl' not found in the program data.");
            }

            let contains_idl_create_account = out_data
                .windows("IdlCreateAccount".len())
                .any(|window| window == b"IdlCreateAccount");
            if contains_idl_create_account {
                println!("Found 'IdlCreateAccount' in the program data.");
            } else {
                println!("'IdlCreateAccount' not found in the program data.");
            }

            if contains_anchor_idl && contains_idl_create_account {
                println!("Both 'anchor:idl' and 'IdlCreateAccount' found. Checking IDL account...");

                let (program_signer, _) = Pubkey::find_program_address(&[], &program_pubkey);

                let idl_account = Pubkey::create_with_seed(&program_signer, "anchor:idl", &program_pubkey)
                    .expect("Seed derivation failed");

                println!("IDL Account Address: {}", idl_account);

                let rpc_url = "https://api.mainnet-beta.solana.com";
                let client = RpcClient::new(rpc_url.to_string());

                match client.get_account(&idl_account) {
                    Ok(account) => {
                        println!("IDL Account Found!");
                        println!("Owner: {}", account.owner);
                        println!("Lamports: {}", account.lamports);
                        println!("Data Length: {}", account.data.len());
                    }
                    Err(err) => {
                        println!("possibly VULNERABLE - IDL Account Not Found: {}", err);
                    }
                }
            } else {
                println!("Probably not Vulnerable - Skipping IDL account check. (no anchor:idl/idlCreateAccount) found");
            }

            if let Err(err) = fs::remove_file(&out_file) {
                eprintln!("Failed to delete the dump file: {}", err);
            } else {
                println!("Dump file deleted successfully.");
            }
        }
        Err(err) => {
            eprintln!("Failed to dump the program: {}", err);
        }
    }
}
