use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    let param = if args.len() > 1 {
        &args[1]
    } else {
        panic!("Please provide the required parameter");
    };

    println!("param: {}", param);
}
