//! bm CLI — закладки с тегами.
//!
//!   bm init <db>
//!   bm add <db> <url> [--tag T]... [--title S] [--note S] [--ts N]
//!   bm find <db> "X-Query"
//!   bm ls <db>
//!   bm tags <db>
//!   bm open <db> <query>     (xdg-open первый результат)
//!   bm pick <db> [query]     (fzf-выбор url)

use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let code = match args.get(1).map(|s| s.as_str()) {
        Some("init") => match args.get(2) {
            Some(db) => match std::fs::create_dir_all(db) {
                Ok(()) => {
                    println!("init {}", db);
                    0
                }
                Err(e) => err(&e.to_string()),
            },
            None => err("init: need db dir"),
        },
        Some("add") => cmd_add(&args[2..]),
        Some("find") => cmd_find(&args[2..]),
        Some("ls") => cmd_ls(&args[2..]),
        Some("tags") => cmd_tags(&args[2..]),
        Some("open") => cmd_open(&args[2..]),
        Some("pick") => cmd_pick(&args[2..]),
        _ => {
            eprintln!("bm: см. usage в шапке исходника");
            2
        }
    };
    std::process::exit(code);
}

fn flag<'a>(a: &'a [String], name: &str) -> Option<&'a String> {
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1))
}

fn cmd_add(a: &[String]) -> i32 {
    let db = match a.get(0).map(PathBuf::from) {
        Some(p) => p,
        None => return err("add: need db"),
    };
    let url = match a.get(1) {
        Some(u) => u.clone(),
        None => return err("add: need url"),
    };
    let title = flag(a, "--title").cloned().unwrap_or_default();
    let note = flag(a, "--note").cloned().unwrap_or_default();
    let ts: u64 = flag(a, "--ts").and_then(|s| s.parse().ok()).unwrap_or_else(now);
    let mut tags = Vec::new();
    let mut i = 2;
    while i < a.len() {
        if a[i] == "--tag" {
            if let Some(t) = a.get(i + 1) {
                tags.push(t.clone());
                i += 2;
                continue;
            }
        }
        i += 1;
    }
    match bm::add(&db, &url, &title, &note, &tags, ts) {
        Ok(id) => {
            println!("added {id}  [{}{}]", url, if tags.is_empty() { String::new() } else { format!(", tags={}", tags.join(",")) });
            0
        }
        Err(e) => err(&e),
    }
}

fn cmd_find(a: &[String]) -> i32 {
    let db = match a.get(0).map(PathBuf::from) {
        Some(p) => p,
        None => return err("find: need db"),
    };
    let q = match a.get(1) {
        Some(q) => q.clone(),
        None => return err("find: need X-Query"),
    };
    match bm::find(&db, &q) {
        Ok(res) => {
            for (url, title, tags) in &res {
                let t = if title.is_empty() { String::new() } else { format!("  ({title})") };
                println!("  {}  [{}]{}", url, tags.join(","), t);
            }
            println!("{} match(es)", res.len());
            0
        }
        Err(e) => err(&e),
    }
}

fn cmd_ls(a: &[String]) -> i32 {
    let db = match a.get(0).map(PathBuf::from) {
        Some(p) => p,
        None => return err("ls: need db"),
    };
    match bm::find(&db, "X-Record-ID != \"\"") {
        Ok(res) => {
            for (url, title, tags) in &res {
                let t = if title.is_empty() { String::new() } else { format!("  ({title})") };
                println!("  {}  [{}]{}", url, tags.join(","), t);
            }
            println!("{} record(s)", res.len());
            0
        }
        Err(e) => err(&e),
    }
}

fn cmd_tags(a: &[String]) -> i32 {
    let db = match a.get(0).map(PathBuf::from) {
        Some(p) => p,
        None => return err("tags: need db"),
    };
    match bm::tag_counts(&db) {
        Ok(c) => {
            for (tag, n) in &c {
                println!("  {:>4}  {}", n, tag);
            }
            0
        }
        Err(e) => err(&e),
    }
}

fn cmd_open(a: &[String]) -> i32 {
    let db = match a.get(0).map(PathBuf::from) {
        Some(p) => p,
        None => return err("open: need db"),
    };
    let q = match a.get(1) {
        Some(q) => q.clone(),
        None => return err("open: need query"),
    };
    match bm::find(&db, &q) {
        Ok(res) => match res.first() {
            Some((url, _, _)) => {
                match std::process::Command::new("xdg-open").arg(url).spawn() {
                    Ok(_) => {
                        println!("opening {url}");
                        0
                    }
                    Err(_) => {
                        println!("{url}");
                        0
                    }
                }
            }
            None => {
                println!("no matches");
                0
            }
        },
        Err(e) => err(&e),
    }
}

fn cmd_pick(a: &[String]) -> i32 {
    let db = match a.get(0).map(PathBuf::from) {
        Some(p) => p,
        None => return err("pick: need db"),
    };
    let q = a.get(1).cloned().unwrap_or_else(|| "X-Record-ID != \"\"".to_string());
    match bm::find(&db, &q) {
        Ok(res) => {
            let urls: Vec<String> = res.into_iter().map(|(u, t, _)| {
                if t.is_empty() { u } else { format!("{u}  ({t})") }
            }).collect();
            if urls.is_empty() {
                println!("no matches");
                return 0;
            }
            if let Some(fzf) = find_fzf() {
                match run_fzf(&fzf, &urls) {
                    Ok(Some(sel)) => {
                        let url = sel.split_whitespace().next().unwrap_or(&sel);
                        println!("{url}");
                    }
                    Ok(None) => {}
                    Err(e) => return err(&e),
                }
            } else {
                for u in &urls {
                    println!("{u}");
                }
            }
            0
        }
        Err(e) => err(&e),
    }
}

fn find_fzf() -> Option<String> {
    for dir in std::env::var("PATH").unwrap_or_default().split(':') {
        let p = format!("{dir}/fzf");
        if std::path::Path::new(&p).is_file() {
            return Some(p);
        }
    }
    None
}

fn run_fzf(fzf: &str, items: &[String]) -> Result<Option<String>, String> {
    use std::io::Write;
    let mut child = std::process::Command::new(fzf)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    {
        let mut stdin = child.stdin.take().ok_or("no stdin")?;
        for it in items {
            writeln!(stdin, "{it}").map_err(|e| e.to_string())?;
        }
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    let sel = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Ok(if sel.is_empty() { None } else { Some(sel) })
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn err(e: &str) -> i32 {
    eprintln!("bm: {e}");
    1
}
