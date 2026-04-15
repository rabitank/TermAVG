use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use anyhow::Context;

pub fn preparse_script(origin_path: &PathBuf, target_path: &PathBuf, origin_number: Option<u32>) -> anyhow::Result<()>{
    let file = File::open(origin_path).context(format!("无法打开文件 {}", &origin_path.to_str().unwrap()))?;
    let reader = BufReader::new(file);

    let mut n: u32 = origin_number.unwrap_or(1); // 起始数字，可根据需要调整
    let mut w_file = File::create(target_path)?;

    for line in reader.lines() {
        let line = line.context(format!("读取行失败"))?;
        if line.starts_with('#') {
            // 去掉开头的 '#', 获取剩余部分
            let rest = &line[1..];
            // 检查剩余部分是否以数字开头
            let first_char = rest.chars().next();
            if let Some(c) = first_char {
                if c.is_ascii_digit() {
                    // 提取数字
                    let mut num_str = String::new();
                    for ch in rest.chars() {
                        if ch.is_ascii_digit() {
                            num_str.push(ch);
                        } else {
                            break;
                        }
                    }
                    if let Ok(num) = num_str.parse::<u32>() {
                        n = num; // 更新当前数字
                    }
                    // 原样输出该行（不做修改）
                    writeln!(w_file, "{}", line).unwrap();
                    continue;
                }
            }
            // 普通 # 行：在 # 后面插入当前 n
            writeln!(w_file, "#{}{}", n, rest).unwrap();
            n += 1;
        } else {
            // 非 # 开头的行原样输出
            writeln!(w_file, "{}", line).unwrap();
        }
    }
    Ok(())
}
