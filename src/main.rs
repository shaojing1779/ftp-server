use std::io::{Error, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::{thread};
use std::time;
use std::env;
use rand::Rng;

// Request type
enum Cmdlist 
{ 
  ABOR(u16), CWD(u16), DELE(u16), LIST(u16), MDTM(u16), MKD(u16), NLST(u16), PASS(u16), PASV(u16),
  PORT(u16), PWD(u16), QUIT(u16), RETR(u16), RMD(u16), RNFR(u16), RNTO(u16), SITE(u16), SIZE(u16),
  STOR(u16), TYPE(u16), CDUP(u16), USER(u16), NOOP(u16), SYST
}

// Request command
struct Command
{
    command: String,
    arg: String,
}

// user connect info
struct State
{
    // Connection mode: 0-NORMAL, 1-SERVER, 2-CLIENT
    mode: i8,
    user_name: String,
    message: String,
    /* PASV MOD*/
    sock_pasv: u32,
    /* PORT MOD*/
    sock_port: u32,
    /* Transport type 0-bin 1-ascii */
    trans_type: i8,
}

static CMD_LIST_VALUE: &'static [&str] = &[
    "ABOR", "CWD", "DELE", "LIST", "MDTM", "MKD", "NLST", "PASS", "PASV",
    "PORT", "PWD", "QUIT", "RETR", "RMD", "RNFR", "RNTO", "SITE", "SIZE",
    "STOR", "TYPE", "CDUP", "USER", "NOOP", "SYST"];

// USER
fn ftp_user() -> String {
    "331 User name okay, need password\n".to_owned()
}

// PASS
fn ftp_pass() -> String {
    "230 Login successful\n".to_owned()
}

// PASV
fn ftp_pasv(state: &State) -> String {
    let mut tu_port:(u16, u16) = (0, 0);

    let mut rng = rand::thread_rng();

    let seed: u16 = rng.gen();
    tu_port.0 = 0b10000000 + seed % 0b1000000;
    tu_port.1 = seed % 0xff;

    // let mut ip: [u32; 4] = [0, 0, 0, 0];

    let port = (0x100 * tu_port.0) + tu_port.1;
    let addr = String::from("0.0.0.0:").to_string() + &port.to_string();
    let listener = TcpListener::bind(addr).unwrap();

    "".to_owned()
}


fn get_pwd() -> String {
    let res = env::current_dir();
    match res {
        Ok(path) => path.into_os_string().into_string().unwrap(),
        Err(_) => "FAILED".to_string()
    }
}
// PWD
fn ftp_pwd() -> String {
    get_pwd().to_owned() + "\n"
}

// LIST
fn ftp_list() -> String {
    get_pwd().to_owned() + "\n"
}

// SYST
fn ftp_syst() -> String {
    "200 🐶🐶 \n".to_owned()
}

// STOR
fn ftp_stor() -> String {

    "".to_owned()
}

fn handle_client(mut stream: TcpStream) -> Result<(), Error>{
    let mut buf = [0; 512];
    
    let welcome = "200 Welcome to FTP service.\n";
    stream.write(welcome.as_bytes())?;

    let state = State {
        user_name : "".to_owned(),
        mode: 0,
        message: "".to_owned(),
        sock_pasv: 0,
        sock_port: 0,
        trans_type: 0,
    };

    loop {
        let bytes_read = stream.read(&mut buf)?;
        if bytes_read == 0 {
            return Ok(());
        }

        let mut cmd = Command {
            command : String::from(""),
            arg : String::from(""),
        };
        let mut t_cmd:Vec<u8> = Vec::new();

        for it in buf {
            if (it == 0x20 || it == 0x0A || it == 0x0D) && cmd.command.is_empty() {
                cmd.command = String::from_utf8(t_cmd.clone()).unwrap();  //char[] -> string
                t_cmd.clear();
                continue;
            } else if it != 0x0 {
                t_cmd.push(it);
            }
        }

        cmd.arg = String::from_utf8(t_cmd.clone()).unwrap();

        println!("split_test:{}, {}", cmd.command, cmd.arg);
        let mut w_buf = String::new();
        match &cmd.command as &str {
            "USER" => w_buf = ftp_user(),
            "PASS" => w_buf = ftp_pass(),
            "PWD" => w_buf = ftp_pwd(),
            "PASV" => w_buf = ftp_pasv(&state),
            "SYST" => w_buf = ftp_syst(),
            "LIST" => w_buf = ftp_list(),
            "STOR" => w_buf = ftp_stor(),
            _=>println!("commond invalid!"),
        }
        stream.write(&w_buf.as_bytes())?;
        thread::sleep(time::Duration::from_secs(1 as u64));
    }

}


fn server(port: &u32)  -> Result<(), Error> {

    let addr = String::from("0.0.0.0:").to_string() + &port.to_string();
    let listener = TcpListener::bind(addr).unwrap();
    let mut v_thread: Vec<thread::JoinHandle<()>> = Vec::new();

    for stream in listener.incoming() {
        let stream = stream.expect("failed!");
        let handle = thread::spawn(move || {
            handle_client(stream)
        .unwrap_or_else(|error| eprintln!("{:?}", error));
        });
        v_thread.push(handle);
    }

    for handle in v_thread {
        handle.join().unwrap();
    }
    Ok(())
}

fn main() -> std::io::Result<()> {

    let args: Vec<String> = env::args().collect();
    let mut port: u32= 2022;
    if args.len() > 1 {
        port = args[1].parse().unwrap();
    }
    let err = server(&port);

    Ok(())
}