use clap::{
    crate_description, crate_name, crate_version, value_t_or_exit, App, AppSettings, Arg,
    SubCommand,
};
use console::Emoji;
use solana_clap_utils::{
    input_parsers::{pubkey_of_signer, signer_of},
    input_validators::{is_amount, is_url, is_valid_pubkey, is_valid_signer},
    keypair::DefaultSigner,
};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, instruction::Instruction, native_token::*, pubkey::Pubkey,
    signature::Signer, transaction::Transaction,
};
use spl_cpi_test::{self, instruction::*};
use std::process::exit;

struct Config {
    rpc_client: RpcClient,
    owner: Pubkey,
    fee_payer: Pubkey,
    commitment_config: CommitmentConfig,
    default_signer: DefaultSigner,
}

type Error = Box<dyn std::error::Error>;
type CommandResult = Result<Option<(u64, Vec<Instruction>)>, Error>;

fn check_fee_payer_balance(config: &Config, required_balance: u64) -> Result<(), Error> {
    let balance = config.rpc_client.get_balance(&config.fee_payer)?;
    if balance < required_balance {
        Err(format!(
            "Fee payer, {}, has insufficient balance: {} required, {} available",
            config.fee_payer,
            lamports_to_sol(required_balance),
            lamports_to_sol(balance)
        )
        .into())
    } else {
        Ok(())
    }
}

fn command_transfer(config: &Config, recipient: Pubkey, amount: f64) -> CommandResult {
    println!(
        "Transfer {} tokens\n  Sender: {}\n  Recipient: {}",
        amount, config.owner, recipient
    );

    let lamports = sol_to_lamports(amount);

    let instructions = vec![invoked_transfer(
        &spl_cpi_test::id(),
        &config.owner,
        &recipient,
        lamports,
    )?];
    Ok(Some((0, instructions)))
}

fn main() {
    let matches = App::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg({
            let arg = Arg::with_name("config_file")
                .short("C")
                .long("config")
                .value_name("PATH")
                .takes_value(true)
                .global(true)
                .help("Configuration file to use");
            if let Some(ref config_file) = *solana_cli_config::CONFIG_FILE {
                arg.default_value(&config_file)
            } else {
                arg
            }
        })
        .arg(
            Arg::with_name("json_rpc_url")
                .long("url")
                .value_name("URL")
                .takes_value(true)
                .global(true)
                .validator(is_url)
                .help("JSON RPC URL for the cluster.  Default from the configuration file."),
        )
        .arg(
            Arg::with_name("owner")
                .long("owner")
                .value_name("KEYPAIR")
                .validator(is_valid_signer)
                .takes_value(true)
                .global(true)
                .help(
                    "Specify the token owner account. \
                     This may be a keypair file, the ASK keyword. \
                     Defaults to the client keypair.",
                ),
        )
        .arg(
            Arg::with_name("fee_payer")
                .long("fee-payer")
                .value_name("KEYPAIR")
                .validator(is_valid_signer)
                .takes_value(true)
                .global(true)
                .help(
                    "Specify the fee-payer account. \
                     This may be a keypair file, the ASK keyword. \
                     Defaults to the client keypair.",
                ),
        )
        .subcommand(
            SubCommand::with_name("transfer")
                .about("Transfer SOL between accounts using cpi")
                .arg(
                    Arg::with_name("recipient")
                        .validator(is_valid_pubkey)
                        .value_name("RECIPIENT_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The account address of the recipient"),
                )
                .arg(
                    Arg::with_name("amount")
                        .validator(is_amount)
                        .value_name("TOKEN_AMOUNT")
                        .takes_value(true)
                        .index(2)
                        .required(true)
                        .help("Amount to send, in tokens"),
                ),
        )
        .get_matches();

    let mut wallet_manager = None;
    let mut bulk_signers: Vec<Option<Box<dyn Signer>>> = Vec::new();

    let config = {
        let cli_config = if let Some(config_file) = matches.value_of("config_file") {
            solana_cli_config::Config::load(config_file).unwrap_or_default()
        } else {
            solana_cli_config::Config::default()
        };
        let json_rpc_url = matches
            .value_of("json_rpc_url")
            .unwrap_or(&cli_config.json_rpc_url)
            .to_string();

        let default_signer_arg_name = "owner".to_string();
        let default_signer_path = matches
            .value_of(&default_signer_arg_name)
            .map(|s| s.to_string())
            .unwrap_or(cli_config.keypair_path);
        let default_signer = DefaultSigner {
            path: default_signer_path,
            arg_name: default_signer_arg_name,
        };
        let owner = default_signer
            .signer_from_path(&matches, &mut wallet_manager)
            .unwrap_or_else(|e| {
                eprintln!("error: {}", e);
                exit(1);
            })
            .pubkey();
        bulk_signers.push(None);
        let (signer, fee_payer) = signer_of(&matches, "fee_payer", &mut wallet_manager)
            .unwrap_or_else(|e| {
                eprintln!("error: {}", e);
                exit(1);
            });
        let fee_payer = fee_payer.unwrap_or(owner);
        bulk_signers.push(signer);

        Config {
            rpc_client: RpcClient::new(json_rpc_url),
            owner,
            fee_payer,
            commitment_config: CommitmentConfig::single_gossip(),
            default_signer,
        }
    };

    solana_logger::setup_with_default("solana=info");

    let _ = match matches.subcommand() {
        ("transfer", Some(arg_matches)) => {
            let recipient = pubkey_of_signer(arg_matches, "recipient", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let amount = value_t_or_exit!(arg_matches, "amount", f64);
            command_transfer(&config, recipient, amount)
        }
        _ => unreachable!(),
    }
    .and_then(|transaction_info| {
        if let Some((minimum_balance_for_rent_exemption, instructions)) = transaction_info {
            let mut transaction =
                Transaction::new_with_payer(&instructions, Some(&config.fee_payer));
            let (recent_blockhash, fee_calculator) = config
                .rpc_client
                .get_recent_blockhash()
                .unwrap_or_else(|e| {
                    eprintln!("error: {}", e);
                    exit(1);
                });
            check_fee_payer_balance(
                &config,
                minimum_balance_for_rent_exemption
                    + fee_calculator.calculate_fee(&transaction.message()),
            )?;
            let signer_info = config
                .default_signer
                .generate_unique_signers(bulk_signers, &matches, &mut wallet_manager)
                .unwrap_or_else(|e| {
                    eprintln!("error: {}", e);
                    exit(1);
                });
            transaction.sign(&signer_info.signers, recent_blockhash);
            println!("{:?}", transaction);

            let signature = config
                .rpc_client
                .send_and_confirm_transaction_with_spinner_and_commitment(
                    &transaction,
                    config.commitment_config,
                )?;
            println!("Signature: {}", signature);
        }
        Ok(())
    })
    .map_err(|err| {
        eprintln!("{}", err);
        exit(1);
    });
}
