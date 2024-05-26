#![allow(unused_imports, dead_code, unused_variables)]
//basic asm for cbvm
use std::io::{Read, Write};
use cbvm::builder::bytes::{Byte, ByteStream};
use cbvm::{stream, byte, typed, op, constant};
use cbvm::bytecode::ops::*;
use cbvm::bytecode::types::*;

//i want a basic asm lexer and parser, like extremely basic
#[derive(Debug)]
struct Token {
    func: String,
    args: Vec<String>,
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} {:?}", self.func, self.args)
    }
}

fn lines(input: String) -> Vec<String> {
    input.split("\n").map(|x| x.trim().to_string()).collect()
}
fn lex_line(input: String) -> Token {
    //if it starts with : it's a label
    if input.starts_with(":") {
        return Token {
            func: "FUNC".to_string(),
            args: vec![input],
        }
    }
    let mut tokens: Vec<String> = input.split(" ").map(|x| x.to_string()).collect();
    let func = tokens.remove(0);
    Token {
        func,
        args: tokens,
    }
}
fn lex(input: String) -> Vec<Token> {
    lines(input).iter().map(|x| lex_line(x.to_string())).collect()
}

//all numbers are by default TypeU64
//TypeReg is represented by [Reg]
//TypeStackDeref is represented by (num), can have reg +/- num
//TypeHeapDeref is represented by {num}, can have reg +/- num
//function to convert args to types
#[derive(Debug)]
enum ArgType {
    U64,
    Reg,
    StackDeref,
    HeapDeref,
    Label,
    Jmp
}
impl From<ArgType> for Types {
    fn from(arg: ArgType) -> Types {
        match arg {
            ArgType::U64 => Types::TypeU64,
            ArgType::Reg => Types::TypeReg,
            ArgType::StackDeref => Types::DerefStack,
            ArgType::HeapDeref => Types::DerefHeapReg,
            ArgType::Label => Types::TypeFunc,
            ArgType::Jmp => Types::TypeJmp,
        }
    }
}
#[derive(Debug)]
struct Arg {
    arg: String,
    arg_type: ArgType,
}
#[derive(Debug)]
struct Branch {
    func: String,
    args: Vec<Arg>,
}

//when data is in [] () or {}, remove the brackets 
fn parse_line(input: Token) -> Branch {
    //var to store reg names and their numbers
    let func = input.func;
    let args = input.args.iter().map(|x| {
        let mut arg = x.to_string();
        let arg_type = if arg.starts_with("[") && arg.ends_with("]") {
            arg.retain(|c| c != '[' && c != ']');
            ArgType::Reg
        } else if arg.starts_with("(") && arg.ends_with(")") {
            arg.retain(|c| c != '(' && c != ')');
            ArgType::StackDeref
        } else if arg.starts_with("{") && arg.ends_with("}") {
            arg.retain(|c| c != '{' && c != '}');
            ArgType::HeapDeref
        } else if arg.starts_with(":") { 
            arg.retain(|c| c != ':');
            ArgType::Label
        } else if arg.starts_with(";") {
            arg.retain(|c| c != ';');
            ArgType::Jmp
    
        } else {
            //turn hex into base 10 u64
            arg = u64::from_str_radix(&*arg, 16).unwrap().to_string();
            ArgType::U64
        };
        Arg {
            arg,
            arg_type,
        }
    }).collect();
    Branch {
        func,
        args,
    }
}
fn parse(input: Vec<Token>) -> Vec<Branch> {
    let mut branches: Vec<Branch> = Vec::new();
    //find every register and assign it a number in the registers vec, if it isn't already there
    for i in input {
        branches.push(parse_line(i));
    }
    let mut registers: Vec<String> = Vec::new();
    let mut functions: Vec<String> = Vec::new();
    //replace any arguments that are registers with their number, and any functions with their number
    for i in branches.iter_mut() {
        for j in i.args.iter_mut() {
            match j.arg_type {
                ArgType::Reg => {
                    if !registers.contains(&j.arg) {
                        registers.push(j.arg.clone());
                    }
                    j.arg = registers.iter().position(|x| x == &j.arg).unwrap().to_string();
                }
                ArgType::Label => {
                    if !functions.contains(&j.arg) {
                        functions.push(j.arg.clone());
                    }
                    j.arg = functions.iter().position(|x| x == &j.arg).unwrap().to_string();
                }
                ArgType::Jmp => { //do the same as label
                    if !functions.contains(&j.arg) {
                        functions.push(j.arg.clone());
                    }
                    j.arg = functions.iter().position(|x| x == &j.arg).unwrap().to_string();
                }
                _ => {}
            }
        }
    }
    branches
}

pub fn build(code: String) -> ByteStream {
    let lexed = lex(code);
    let parsed = parse(lexed);
    let mut compiler = Compiler::new();
    compiler.compile(parsed);
    compiler.bytecode
}

//the builder, creates the bytecode
struct Compiler {
    //something to hold register names and associate with numbers
    registers: Vec<String>, //index of register is the register number
    //something to hold the bytecode
    bytecode: ByteStream,
    //labels, and associted line numbers
    labels: Vec<String>, //index of label is the number given to bytecode
}

impl Compiler {
    fn new() -> Compiler {
        Compiler {
            registers: Vec::new(),
            bytecode: ByteStream::new(),
            labels: Vec::new(),
        }
    }
    fn compile(&mut self, input: Vec<Branch>) {
        for i in input {
            match i.func.as_str() {
                "ALLOC" => {
                    self.bytecode = self.bytecode.emit(op!(ALLOC));
                    //for each argument, emit a byte
                    for i in i.args {
                        self.bytecode = self.bytecode.emit(Byte {
                            data: Box::from(i.arg.parse::<u64>().unwrap()),
                            pos: 0,
                            tp: Types::from(i.arg_type),
                        });
                    }
                }
                "STORE" => {
                    self.bytecode = self.bytecode.emit(op!(STORE));
                    for i in i.args {
                        self.bytecode = self.bytecode.emit(Byte {
                            data: Box::from(i.arg.parse::<u64>().unwrap()),
                            pos: 0,
                            tp: Types::from(i.arg_type),
                        });
                    }
                }
                "WRITE" => {
                    self.bytecode = self.bytecode.emit(op!(WRITE));
                    for i in i.args {
                        self.bytecode = self.bytecode.emit(Byte {
                            data: Box::from(i.arg.parse::<u64>().unwrap()),
                            pos: 0,
                            tp: Types::from(i.arg_type),
                        });
                    }
                }
                "FLUSH" => {
                    self.bytecode = self.bytecode.emit(op!(FLUSH));
                }
                "FREE" => {
                    self.bytecode = self.bytecode.emit(op!(FREE));
                    for i in i.args {
                        self.bytecode = self.bytecode.emit(Byte {
                            data: Box::from(i.arg.parse::<u64>().unwrap()),
                            pos: 0,
                            tp: Types::from(i.arg_type),
                        });
                    }
                }
                "ADD" => {
                    self.bytecode = self.bytecode.emit(op!(ADD));
                    for i in i.args {
                        self.bytecode = self.bytecode.emit(Byte {
                            data: Box::from(i.arg.parse::<u64>().unwrap()),
                            pos: 0,
                            tp: Types::from(i.arg_type),
                        });
                    }
                }
                "FUNC" => {
                    self.bytecode = self.bytecode.emit(op!(FUNC));
                    println!("Function: {:?}", i.args[0]);
                    for i in i.args {
                        self.bytecode = self.bytecode.emit(Byte {
                            data: Box::from(i.arg.parse::<u64>().unwrap()),
                            pos: 0,
                            tp: Types::from(i.arg_type),
                        });
                    }
                }
                "SUB" => {
                    self.bytecode = self.bytecode.emit(op!(SUB));
                    for i in i.args {
                        self.bytecode = self.bytecode.emit(Byte {
                            data: Box::from(i.arg.parse::<u64>().unwrap()),
                            pos: 0,
                            tp: Types::from(i.arg_type),
                        });
                    }
                }
                "JMP" => {
                    self.bytecode = self.bytecode.emit(op!(JMP));
                    for i in i.args {
                        self.bytecode = self.bytecode.emit(Byte {
                            data: Box::from(i.arg.parse::<u64>().unwrap()),
                            pos: 0,
                            tp: Types::from(i.arg_type),
                        });
                    }
                }
                _ => {
                    println!("Unknown function: {}", i.func);
                }
            }
        }
    }
}

fn main() {
    //take cli args, <optionss> <file>, options are -o <output file>, -d (disassemble), -c (compile (assumed))
    let args: Vec<String> = std::env::args().collect();
    let mut input = String::new();
    let mut output = String::new();
    let mut disassemble = false;
    let mut compile = true;
    for i in 1..args.len() {
        match args[i].as_str() {
            "-o" => {
                output = args[i + 1].to_string();
            }
            "-d" => {
                disassemble = true;
                compile = false;
            }
            "-c" => {
                compile = true;
                disassemble = false;
            }
            _ => {
                input = args[i].to_string();
            }
        }
    }
    //if no input file, print usage
    if input.is_empty() {
        println!("Usage: cbasm <options> <file>");
        println!("Options:");
        println!("\t-o <output file> - specify output file");
        println!("\t-d - disassemble");
        println!("\t-c - compile");
        std::process::exit(1);
    }
    //if no output file, use input file with .cb extension
    if output.is_empty() {
        output = format!("{}.cb", input.split(".").collect::<Vec<&str>>()[0]);
    }
    //if compile, read file, lex, parse, compile, write to output file
    if compile {
        //open file, read contents, lex, parse, compile
        let mut file = std::fs::File::open(&input).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        let lexed = lex(contents);
        let parsed = parse(lexed);
        println!("{:?}", parsed);
        let mut compiler = Compiler::new();
        compiler.compile(parsed);
        let mut output_file = std::fs::File::create(&output).unwrap();
        write!(output_file, "{}", compiler.bytecode.stringify()).unwrap();

    }
    //if disassemble, read file, disassemble, write to output file
    if disassemble {
        let mut output_file = std::fs::File::create(&(output + "asm")).unwrap();
        let mut reader = cbvm::reader::Reader::new(&input);
        reader.read();
        reader.group();
        let disassembled = cbvm::asm::mkasm(reader.bytes);
        output_file.write_all(disassembled.as_bytes()).unwrap();
    }
}