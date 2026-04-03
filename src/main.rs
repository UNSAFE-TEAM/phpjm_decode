use base64::Engine as _;
use regex::bytes::Regex;
use std::error::Error;
use std::fs;
use std::path::Path;
use zlib_rs::{InflateConfig, ReturnCode, decompress_slice};

fn print_banner() {
    println!(" _   _ _   _ ____    _    _____ _____    _____ _____    _    __  __ ");
    println!("| | | | \\ | / ___|  / \\  |  ___| ____|  |_   _| ____|  / \\  |  \\/  |");
    println!("| | | |  \\| \\___ \\ / _ \\ | |_  |  _| _____| | |  _|   / _ \\ | |\\/| |");
    println!("| |_| | |\\  |___) / ___ \\|  _| | |__|_____| | | |___ / ___ \\| |  | |");
    println!(" \\___/|_| \\_|____/_/   \\_\\_|   |_____|    |_| |_____/_/   \\_\\_|  |_|");
    println!();
    println!("  phpjm_decode  |  @Github UNSAFE-TEAM");
    println!("{}", "- ".repeat(35));
}

fn read_file_to_hex<P: AsRef<Path>>(path: P) -> String {
    fs::read(path)
        .expect("read file error")
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

fn filter_base64(data: &[u8]) -> Vec<u8> {
    let re = Regex::new(r"(?-u)[^A-Za-z0-9+/=]").unwrap();
    re.replace_all(data, &b""[..]).to_vec()
}

fn decode(data: &str, key: &str) -> String {
    let mut data_bytes = hex::decode(data).expect("invalid hex input");
    let key_bytes = hex::decode(key).expect("invalid hex key");

    let mut reversed_key = key_bytes.clone();
    reversed_key.reverse();

    // strtr($var1, $var2, strrev($var2));
    for byte in data_bytes.iter_mut() {
        if let Some(pos) = key_bytes.iter().position(|&x| x == *byte) {
            *byte = reversed_key[pos];
        }
    }

    // 嵌套func0 - base64_decode
    hex::encode(
        base64::engine::general_purpose::STANDARD
            .decode(&filter_base64(&data_bytes))
            .expect("base64_decode fragment error"),
    )
}

fn parser_fragment_a(content: &[u8]) -> Option<(String, String)> {
    let re = Regex::new(r"a02822(?P<data>[a-f0-9]+?)222c22(?P<key>[a-f0-9]+?)22293b")
        .expect("parser fragment a error");
    re.captures_iter(content).last().map(|caps| {
        (
            String::from_utf8_lossy(&caps["data"]).to_string(),
            String::from_utf8_lossy(&caps["key"]).to_string(),
        )
    })
}
fn parser_fragment_b(content: &[u8]) -> Option<String> {
    let re = Regex::new(r"72657475726e2022(?P<data>[a-f0-9]+?)223b7d7d")
        .expect("parser fragment c error");

    re.captures(content)
        .map(|caps| String::from_utf8_lossy(&caps["data"]).to_string())
}

fn parser_fragment_c(content: &[u8]) -> Option<String> {
    let re = Regex::new(r"2827(?P<data>654e.*?)272929293b22").expect("parser fragment b error");

    re.captures_iter(content)
        .last()
        .map(|caps| String::from_utf8_lossy(&caps["data"]).to_string())
}

fn splicing_data(fragment_c: &str, decoded_a: &str, fragment_b: &str) -> String {
    let re = regex::Regex::new(r"222e24[a-f0-9]+2e22").expect("splicing regex error");
    let replacement = format!("{}{}", decoded_a, fragment_b);

    re.replace_all(fragment_c, replacement.as_str()).to_string()
}

fn decompress(hex_input: &str) -> Result<String, Box<dyn Error>> {
    let bin_data = hex::decode(hex_input)?;

    let decoded_b64 =
        base64::engine::general_purpose::STANDARD.decode(&filter_base64(&bin_data))?;

    let mut decompressed_buf = vec![0u8; decoded_b64.len() * 10];
    let (decompressed, rc) = decompress_slice(
        &mut decompressed_buf,
        &decoded_b64,
        InflateConfig::default(),
    );

    if rc == ReturnCode::Ok {
        Ok(String::from_utf8_lossy(decompressed).into_owned())
    } else {
        Err(format!("zlib-rs decompression failed: {:?}", rc).into())
    }
}

fn main() {
    print_banner();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("用法: {} <file>", args[0]);
        std::process::exit(1);
    }

    let input_path = Path::new(&args[1]);

    if !input_path.exists() {
        eprintln!("错误: 文件不存在 -> {}", input_path.display());
        std::process::exit(1);
    }

    if !input_path.is_file() {
        eprintln!("错误: 路径不是一个文件 -> {}", input_path.display());
        std::process::exit(1);
    }

    let input_path = Path::new(&args[1]);

    let file_hex = read_file_to_hex(input_path);

    let mut vara = parser_fragment_a(file_hex.as_bytes())
        .map(|(data, key)| decode(&data, &key))
        .unwrap();
    let varb = parser_fragment_b(file_hex.as_bytes()).unwrap();
    let varc = parser_fragment_c(file_hex.as_bytes()).unwrap();

    vara = hex::encode(decompress(&vara).unwrap());

    let result = splicing_data(&varc, &vara, &varb);
    let decoded = decompress(&result).unwrap();

    let output_path = {
        let stem = input_path.file_stem().unwrap_or_default().to_string_lossy();
        let ext = input_path.extension().unwrap_or_default().to_string_lossy();
        let new_filename = if ext.is_empty() {
            format!("{}.decode", stem)
        } else {
            format!("{}.decode.{}", stem, ext)
        };
        input_path.with_file_name(new_filename)
    };

    fs::write(&output_path, &decoded).expect("写入文件失败");
    println!("已保存至 {}", output_path.display());
}
