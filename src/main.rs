use dotenv::dotenv;
use clap::{Arg, Command};
use std::env;

mod ipv4;
mod wedos;

fn main() {
    dotenv().ok();
    let arg_matches = Command::new("WedosDnsTool")
        .version("0.0.1")
        .arg(Arg::new("MODE").short('m').long("mode").value_parser(["public-ip", "list-domains", "list-dns-rows", "set-ip"]).required(true))
        .get_matches();

    match arg_matches.get_one::<String>("MODE").expect("'MODE' is required and parsing will fail if its missing").as_str() {
        "public-ip" => {
            let public_ipv4 = ipv4::get_public_ipv4();
            println!("{}", public_ipv4);
        }
        "list-domains" => {
            wedos::list_domains();
        }
        "list-dns-rows" => {
            let domain = env::var("DOMAIN").unwrap();
            wedos::list_dns_rows(domain);

        }
        "set-ip" => {
            let public_ipv4 = ipv4::get_public_ipv4();
            let domain = env::var("DOMAIN").unwrap();
            let row_id = env::var("DNS_ROW_ID").unwrap();
            wedos::update_a_record(public_ipv4, domain, row_id, None);
        }
        _ => unreachable!(),
    }
}
