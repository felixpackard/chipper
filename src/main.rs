use interpreter::Interpreter;

mod interpreter;
mod memory;

fn main() {
    let mut interpreter = Interpreter::new();
    interpreter.load_font().unwrap();
    println!("{}", interpreter);
}
