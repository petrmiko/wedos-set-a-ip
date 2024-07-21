use dotenv::dotenv;

mod ipv4;
mod wedos;

fn main() {
    dotenv().ok();
    let public_ipv4 = ipv4::get_public_ipv4();
    println!("Public IP is {}", public_ipv4);
    wedos::update_a_record(public_ipv4);
}
