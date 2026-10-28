pub mod benchmark;
pub mod render_bench;
use std::{fs, path::Path};

fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path)
        .unwrap_or_default()
        .trim()
        .to_owned()
}
fn value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines().find_map(|line| {
        let (k, v) = line.split_once(':')?;
        (k.trim() == key).then_some(v.trim())
    })
}
fn kib_value(input: &str) -> Option<u64> {
    input.split_whitespace().next()?.parse().ok()
}
fn memory_text(meminfo: &str) -> Option<String> {
    let total = value(meminfo, "MemTotal").and_then(kib_value)?;
    let available = value(meminfo, "MemAvailable").and_then(kib_value)?;
    if total == 0 {
        return None;
    }
    let used = total.saturating_sub(available);
    let gib = 1024.0 * 1024.0;
    Some(format!(
        "{:.2} GiB usados / {:.2} GiB ({:.1}%)",
        used as f64 / gib,
        total as f64 / gib,
        used as f64 * 100.0 / total as f64
    ))
}
fn cpu_list(input: &str) -> Option<Vec<usize>> {
    let mut cpus = Vec::new();
    for part in input.trim().split(',') {
        let (start, end) = match part.split_once('-') {
            Some((a, b)) => (a.parse::<usize>().ok()?, b.parse::<usize>().ok()?),
            None => {
                let n = part.parse::<usize>().ok()?;
                (n, n)
            }
        };
        if end < start || end > 1_048_575 {
            return None;
        }
        cpus.extend(start..=end);
    }
    cpus.sort_unstable();
    cpus.dedup();
    if cpus.is_empty() {
        None
    } else {
        Some(cpus)
    }
}
fn unique_cores(ids: &[(i64, i64, i64)]) -> usize {
    ids.iter().collect::<std::collections::BTreeSet<_>>().len()
}
fn physical_cores(root: &Path, cpus: &[usize]) -> Option<usize> {
    let mut ids = Vec::with_capacity(cpus.len());
    for cpu in cpus {
        let topology = root.join(format!("cpu{cpu}/topology"));
        let package = read(topology.join("physical_package_id"))
            .parse::<i64>()
            .ok()
            .filter(|n| *n >= 0)?;
        let core = read(topology.join("core_id"))
            .parse::<i64>()
            .ok()
            .filter(|n| *n >= 0)?;
        let die = read(topology.join("die_id"))
            .parse::<i64>()
            .ok()
            .filter(|n| *n >= 0)
            .unwrap_or(0);
        ids.push((package, die, core));
    }
    let count = unique_cores(&ids);
    (count > 0).then_some(count)
}
fn vdf_value(line: &str, expected: &str) -> Option<String> {
    fn quoted(s: &str) -> Option<(String, &str)> {
        let mut chars = s.trim_start().char_indices();
        if chars.next()?.1 != '\"' {
            return None;
        }
        let mut result = String::new();
        let mut escaped = false;
        for (i, c) in chars {
            if escaped {
                result.push(c);
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '\"' {
                return Some((result, &s.trim_start()[i + 1..]));
            } else {
                result.push(c);
            }
        }
        None
    }
    let (key, rest) = quoted(line)?;
    if key != expected {
        return None;
    }
    Some(quoted(rest)?.0)
}
fn steam_games() -> Vec<String> {
    use std::collections::BTreeSet;
    let Some(home) = std::env::var_os("HOME") else {
        return Vec::new();
    };
    let home = std::path::PathBuf::from(home);
    let roots = [
        home.join(".steam/steam"),
        home.join(".local/share/Steam"),
        home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
    ];
    let mut libraries = BTreeSet::new();
    for root in roots {
        if root.is_dir() {
            libraries.insert(root.clone());
        }
        let content = read(root.join("steamapps/libraryfolders.vdf"));
        for line in content.lines() {
            if let Some(path) = vdf_value(line, "path") {
                libraries.insert(std::path::PathBuf::from(path));
            }
        }
    }
    let mut games = BTreeSet::new();
    for library in libraries {
        if let Ok(entries) = fs::read_dir(library.join("steamapps")) {
            for entry in entries.flatten() {
                let file = entry.file_name().to_string_lossy().into_owned();
                if !file.starts_with("appmanifest_") || !file.ends_with(".acf") {
                    continue;
                }
                let manifest = read(entry.path());
                let name = manifest.lines().find_map(|line| vdf_value(line, "name"));
                let id = manifest.lines().find_map(|line| vdf_value(line, "appid"));
                if let (Some(name), Some(id)) = (name, id) {
                    games.insert(format!("{name} (Steam {id})"));
                }
            }
        }
    }
    games.into_iter().collect()
}
fn sensor_label(driver: &str, label: &str) -> String {
    if driver == "coretemp" {
        if let Some(number) = label.strip_prefix("Core ") {
            return format!("CPU · núcleo {number}");
        }
        return format!("CPU · {}", label.replace('_', " "));
    }
    format!("{} · {}", driver.replace('_', " "), label.replace('_', " "))
}
fn pci_names(data: &str, vendor_id: &str, device_id: &str) -> (Option<String>, Option<String>) {
    let vendor_id = vendor_id.trim().trim_start_matches("0x");
    let device_id = device_id.trim().trim_start_matches("0x");
    let mut vendor = None;
    let mut device = None;
    let mut in_vendor = false;
    for line in data.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if !line.starts_with('\t') {
            if in_vendor {
                break;
            }
            if let Some((id, name)) = line.split_once("  ") {
                if id.eq_ignore_ascii_case(vendor_id) {
                    vendor = Some(name.trim().to_owned());
                    in_vendor = true;
                }
            }
        } else if in_vendor && !line.starts_with("\t\t") {
            if let Some((id, name)) = line[1..].split_once("  ") {
                if id.eq_ignore_ascii_case(device_id) {
                    device = Some(name.trim().to_owned());
                    break;
                }
            }
        }
    }
    (vendor, device)
}
fn display_gpu(card: &str, vendor_id: &str, device_id: &str, driver: &str, data: &str) -> String {
    let (vendor, device) = pci_names(data, vendor_id, device_id);
    let vendor = vendor
        .or_else(|| {
            match vendor_id
                .trim()
                .trim_start_matches("0x")
                .to_ascii_lowercase()
                .as_str()
            {
                "8086" => Some("Intel".to_owned()),
                "1002" => Some("AMD".to_owned()),
                "10de" => Some("NVIDIA".to_owned()),
                _ => None,
            }
        })
        .unwrap_or_else(|| format!("fabricante não identificado ({vendor_id})"));
    let model = device
        .map(|name| match name.as_str() {
            "Xeon E3-1200 v2/3rd Gen Core processor Graphics Controller" => {
                "Controlador gráfico de processadores Core de 3ª geração / Xeon E3-1200 v2"
                    .to_owned()
            }
            _ => name.replace("Graphics Controller", "controlador gráfico"),
        })
        .unwrap_or_else(|| format!("modelo não identificado ({device_id})"));
    let number = card.strip_prefix("card").unwrap_or(card);
    format!("Placa de vídeo {number}: {vendor} · {model} · driver {driver}")
}
fn pci_database() -> &'static str {
    static PCI_IDS: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    PCI_IDS.get_or_init(|| {
        [
            "/usr/share/hwdata/pci.ids",
            "/usr/share/misc/pci.ids",
            "/usr/share/pcids/pci.ids",
        ]
        .into_iter()
        .find_map(|path| std::fs::read_to_string(path).ok())
        .unwrap_or_default()
    })
}
fn core_average(readings: &[(String, String, f64)]) -> Option<(f64, usize)> {
    let mut sum = 0.0;
    let mut count = 0;
    for (driver, label, value) in readings {
        // coretemp's Core N labels are per-core sensors; Package id is not a core.
        let suffix = label.strip_prefix("Core ");
        if driver == "coretemp"
            && suffix.is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
            && value.is_finite()
        {
            sum += value;
            count += 1;
        }
    }
    (count > 0).then(|| (sum / count as f64, count))
}

fn escape_html(input: &str) -> String {
    let mut out = String::new();
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}
fn utc_time(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += if month <= 2 { 1 } else { 0 };
    let rest = seconds % 86_400;
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02} UTC",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReportTheme {
    Light,
    Dark,
}

pub fn report_html(
    snapshot: &str,
    benchmark: Option<&str>,
    generated_seconds: u64,
    theme: ReportTheme,
) -> String {
    let (scheme, background, card, text, muted, border, accent) = match theme {
        ReportTheme::Light => (
            "light", "#f6f6f6", "#ffffff", "#403f53", "#626b7d", "#d8dfe3", "#365f9d",
        ),
        ReportTheme::Dark => (
            "dark", "#011627", "#0b253a", "#d6deeb", "#a6accd", "#1d3b53", "#82aaff",
        ),
    };
    let mut html = format!(
        r#"<!doctype html>
<html lang="pt-BR"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Relatório Reset Explode</title><style>:root{{color-scheme:{scheme};--background:{background};--card:{card};--text:{text};--muted:{muted};--border:{border};--accent:{accent}}}html{{background:var(--background);color:var(--text);print-color-adjust:exact;-webkit-print-color-adjust:exact}}body{{font:16px system-ui,sans-serif;max-width:960px;margin:40px auto;padding:0 20px;line-height:1.5}}header{{border-bottom:2px solid var(--accent);margin-bottom:24px}}section{{background:var(--card);border:1px solid var(--border);border-radius:16px;margin:16px 0;padding:18px 22px}}h2{{font-size:1.1rem;color:var(--accent);margin:0 0 12px}}p{{white-space:pre-wrap;margin:0}}.note{{color:var(--muted);font-size:.9rem}}@media print{{body{{margin:0}}section{{break-inside:avoid}}}}</style></head><body><header><h1>RESET EXPLODE — Relatório</h1>"#
    );
    html.push_str(&format!(
        "<p class=\"note\">Gerado em {} · leituras do momento da exportação</p></header>",
        utc_time(generated_seconds)
    ));
    for section in snapshot.split("\n\n") {
        let mut lines = section.lines();
        let Some(title) = lines.next() else { continue };
        if title.starts_with("RESET EXPLODE") || title.is_empty() {
            continue;
        }
        let body = lines.collect::<Vec<_>>().join("\n");
        html.push_str(&format!(
            "<section><h2>{}</h2><p>{}</p></section>",
            escape_html(title),
            escape_html(&body)
        ));
    }
    html.push_str(&format!("<section><h2>RESULTADOS DA SESSÃO</h2><p>{}</p><p class=\"note\">Resultados disponíveis na sessão; verificações incompletas não determinam a saúde do hardware.</p></section>", escape_html(benchmark.unwrap_or("Nenhum resultado disponível nesta sessão."))));
    html.push_str("</body></html>\n");
    html
}
fn disk_brand(vendor: &str, model: &str) -> String {
    let vendor = vendor.split_whitespace().collect::<Vec<_>>().join(" ");
    let model = model.trim().to_ascii_uppercase();
    let generic = vendor.is_empty()
        || ["ATA", "SATA", "NVME", "USB", "GENERIC"]
            .contains(&vendor.to_ascii_uppercase().as_str())
        || vendor.starts_with("0x");
    if !generic {
        return vendor;
    }
    for (prefix, brand) in [
        ("KINGSTON", "Kingston"),
        ("SAMSUNG", "Samsung"),
        ("SEAGATE", "Seagate"),
        ("TOSHIBA", "Toshiba"),
        ("CRUCIAL", "Crucial"),
        ("INTEL", "Intel"),
        ("SANDISK", "SanDisk"),
        ("WDC", "Western Digital"),
        ("WESTERN DIGITAL", "Western Digital"),
        ("HGST", "HGST"),
        ("MICRON", "Micron"),
        ("KIOXIA", "Kioxia"),
        ("ADATA", "ADATA"),
    ] {
        if model.starts_with(prefix) {
            return brand.to_owned();
        }
    }
    "Fabricante não informado".to_owned()
}
fn disk_rows(root: &Path) -> Vec<String> {
    let mut rows = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        let mut paths: Vec<_> = entries.flatten().map(|x| x.path()).collect();
        paths.sort();
        for path in paths {
            let Some(name) = path.file_name().and_then(|x| x.to_str()) else {
                continue;
            };
            if ["loop", "ram", "zram", "dm-", "md"]
                .iter()
                .any(|prefix| name.starts_with(prefix))
            {
                continue;
            }
            let vendor = read(path.join("device/vendor"));
            let model = read(path.join("device/model"))
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            let brand = disk_brand(&vendor, &model);
            // Linux reports this attribute in fixed 512-byte sectors, independent of device sector size.
            let capacity = read(path.join("size"))
                .parse::<u64>()
                .ok()
                .filter(|sectors| *sectors > 0)
                .map(|sectors| format!("{:.1} GiB", sectors as f64 / 2_097_152.0))
                .unwrap_or_else(|| "capacidade indisponível".to_owned());
            rows.push(format!("/dev/{name}: {brand} · {capacity}"));
        }
    }
    rows
}
fn sensor_readings(root: &Path) -> Vec<(String, String, f64)> {
    let mut readings: Vec<(String, String, f64)> = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        let mut paths: Vec<_> = entries.flatten().map(|x| x.path()).collect();
        paths.sort();
        for path in paths {
            let name = read(path.join("name"));
            if name == "acpitz" {
                continue;
            }
            if let Ok(files) = fs::read_dir(&path) {
                let mut files: Vec<_> = files.flatten().map(|x| x.path()).collect();
                files.sort();
                for file in files {
                    let Some(n) = file.file_name().and_then(|x| x.to_str()) else {
                        continue;
                    };
                    if !n.starts_with("temp") || !n.ends_with("_input") {
                        continue;
                    }
                    if let Ok(raw) = read(&file).parse::<f64>() {
                        let label = read(path.join(n.replace("_input", "_label")));
                        if raw.is_finite()
                            && !(name == "coretemp" && label.starts_with("Package id "))
                        {
                            readings.push((
                                name.clone(),
                                if label.is_empty() {
                                    n.to_owned()
                                } else {
                                    label
                                },
                                raw / 1000.0,
                            ));
                        }
                    }
                }
            }
        }
    }
    readings
}
pub fn snapshot() -> String {
    let os = read("/etc/os-release");
    let name = os
        .lines()
        .find_map(|l| l.strip_prefix("PRETTY_NAME="))
        .unwrap_or("Linux")
        .trim_matches('"');
    let cpuinfo = read("/proc/cpuinfo");
    let cpu = value(&cpuinfo, "model name")
        .or_else(|| value(&cpuinfo, "Hardware"))
        .unwrap_or("indisponível");
    let mem = read("/proc/meminfo");
    let available_threads = std::thread::available_parallelism().ok().map(|x| x.get());
    let online_cpus = cpu_list(&read("/sys/devices/system/cpu/online"));
    let online_threads = online_cpus.as_ref().map(Vec::len);
    let cores = online_cpus
        .as_ref()
        .and_then(|cpus| physical_cores(Path::new("/sys/devices/system/cpu"), cpus))
        .map(|n| n.to_string())
        .unwrap_or_else(|| "indisponível".into());
    let threads = online_threads
        .or(available_threads)
        .map(|n| n.to_string())
        .unwrap_or_else(|| "indisponível".into());
    let restriction = match (online_threads, available_threads) {
        (Some(online), Some(available)) if online != available => {
            format!("\nThreads disponíveis ao aplicativo: {available}")
        }
        _ => String::new(),
    };
    let memory = memory_text(&mem).unwrap_or_else(|| "indisponível".into());
    let mut out = format!("RESET EXPLODE · LINUX\n\nSISTEMA\nSistema: {name}\nKernel: {}\nCPU: {cpu}\nNúcleos: {cores}\nThreads: {threads}{restriction}\nMemória: {memory}\n\n", read("/proc/sys/kernel/osrelease"));
    out.push_str("SENSORES\n");
    let readings = sensor_readings(Path::new("/sys/class/hwmon"));
    match core_average(&readings) {
        Some((mean, _)) => out.push_str(&format!("Temperatura média: {mean:.1} °C\n")),
        None => {
            out.push_str("Temperatura média: indisponível (sensores por núcleo não detectados)\n")
        }
    }
    if readings.is_empty() {
        out.push_str("Nenhum sensor acessível.\n");
    }
    for (name, label, temp) in &readings {
        out.push_str(&format!("{}: {temp:.1} °C\n", sensor_label(name, label)));
    }
    out.push_str("\nARMAZENAMENTO\n");
    let disks = disk_rows(Path::new("/sys/block"));
    if disks.is_empty() {
        out.push_str("Nenhum disco detectado.\n");
    } else {
        for disk in disks {
            out.push_str(&format!("{disk}\n"));
        }
    }
    out.push_str("\nENERGIA\n");
    let mut batteries = 0;
    if let Ok(entries) = fs::read_dir("/sys/class/power_supply") {
        for entry in entries.flatten() {
            let path = entry.path();
            if read(path.join("type")) != "Battery" {
                continue;
            }
            batteries += 1;
            out.push_str(&format!(
                "{}: {}% · {}\n",
                entry.file_name().to_string_lossy(),
                read(path.join("capacity")),
                read(path.join("status"))
            ));
        }
    }
    if batteries == 0 {
        out.push_str("Nenhuma bateria detectada.\n");
    }
    out.push_str("\nPLACA DE VÍDEO\n");
    let mut cards = Vec::new();
    if let Ok(entries) = fs::read_dir("/sys/class/drm") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.starts_with("card") || !name[4..].chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            let p = entry.path().join("device");
            let vendor = read(p.join("vendor"));
            let device = read(p.join("device"));
            let driver = fs::read_link(p.join("driver"))
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "indisponível".into());
            cards.push(display_gpu(
                &name,
                &vendor,
                &device,
                &driver,
                pci_database(),
            ));
        }
    }
    if cards.is_empty() {
        out.push_str("Nenhuma GPU acessível.\n");
    } else {
        cards.sort();
        for card in cards {
            out.push_str(&format!("{card}\n"));
        }
    }
    out.push_str("\nREDE\n");
    let mut interfaces = Vec::new();
    if let Ok(entries) = fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name == "lo" {
                continue;
            }
            let path = entry.path();
            interfaces.push(format!(
                "{name}: {} · recebidos {} bytes · enviados {} bytes",
                read(path.join("operstate")),
                read(path.join("statistics/rx_bytes")),
                read(path.join("statistics/tx_bytes"))
            ));
        }
    }
    if interfaces.is_empty() {
        out.push_str("Nenhuma interface detectada.\n");
    } else {
        interfaces.sort();
        for item in interfaces {
            out.push_str(&format!("{item}\n"));
        }
    }
    out.push_str("\nBIBLIOTECA STEAM\n");
    let games = steam_games();
    if games.is_empty() {
        out.push_str("Nenhum jogo instalado encontrado nas bibliotecas Steam acessíveis.\n");
    } else {
        for game in games {
            out.push_str(&format!("{game}\n"));
        }
    }
    out.push_str("\nPORTAS SERIAIS\n");
    let mut ports = Vec::new();
    if let Ok(entries) = fs::read_dir("/sys/class/tty") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("ttyUSB") || name.starts_with("ttyACM") {
                ports.push(format!("/dev/{name}"));
            }
        }
    }
    if ports.is_empty() {
        out.push_str("Nenhuma porta USB serial detectada.\n");
    } else {
        ports.sort();
        for item in ports {
            out.push_str(&format!("{item}\n"));
        }
    }
    out
}

pub fn page_text(all: &str, page: &str) -> String {
    if page == "Visão geral" {
        return all.to_owned();
    }
    all.split("\n\n")
        .find(|part| part.lines().next() == Some(page))
        .unwrap_or("Dados indisponíveis.")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_values() {
        assert_eq!(
            value("MemTotal: 2048 kB\nMemAvailable: 1024 kB", "MemTotal").and_then(kib_value),
            Some(2048)
        );
        assert_eq!(value("x: 1", "missing"), None);
    }
    #[test]
    fn memory_is_converted_and_percentage_uses_available() {
        assert_eq!(
            memory_text("MemTotal: 8388608 kB\nMemAvailable: 2097152 kB"),
            Some("6.00 GiB usados / 8.00 GiB (75.0%)".to_owned())
        );
        assert_eq!(memory_text("MemTotal: 0 kB\nMemAvailable: 0 kB"), None);
        assert_eq!(memory_text("MemTotal: 8388608 kB"), None);
        assert_eq!(
            memory_text("MemTotal: 1048576 kB\nMemAvailable: 2097152 kB"),
            Some("0.00 GiB usados / 1.00 GiB (0.0%)".to_owned())
        );
    }
    #[test]
    fn cpu_online_list_counts_threads() {
        assert_eq!(cpu_list("0-3").map(|v| v.len()), Some(4));
        assert_eq!(cpu_list("0-3,6,8-9").map(|v| v.len()), Some(7));
        assert_eq!(cpu_list("0-1,1-2").map(|v| v.len()), Some(3));
        assert_eq!(cpu_list(""), None);
        assert_eq!(cpu_list("4-2"), None);
        assert_eq!(
            unique_cores(&[(0, 0, 0), (0, 0, 0), (0, 0, 1), (1, 0, 0)]),
            3
        );
    }
    #[test]
    fn parse_steam_manifest() {
        assert_eq!(
            vdf_value("\t\"name\"\t\"Portal 2\"", "name").as_deref(),
            Some("Portal 2")
        );
        assert_eq!(vdf_value("\"appid\" \"620\"", "name"), None);
    }
    #[test]
    fn page_selection() {
        assert_eq!(
            page_text("SISTEMA\nTeste\n\nREDE\neth0", "REDE"),
            "REDE\neth0"
        );
    }
    #[test]
    fn identifies_disk_brand_without_guessing_unknown_models() {
        assert_eq!(disk_brand("ATA", "KINGSTON SA400S3"), "Kingston");
        assert_eq!(disk_brand("ATA", "WDC WD10"), "Western Digital");
        assert_eq!(disk_brand("Samsung", "XYZ"), "Samsung");
        assert_eq!(
            disk_brand("ATA", "DESCONHECIDO"),
            "Fabricante não informado"
        );
        assert_eq!(disk_brand("0x1234", ""), "Fabricante não informado");
    }
    #[test]
    fn disk_rows_show_path_brand_capacity_and_exclude_virtual_devices() {
        let dir = std::env::temp_dir().join(format!(
            "reset-disk-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(dir.join("sda/device")).unwrap();
        fs::create_dir_all(dir.join("loop0/device")).unwrap();
        fs::write(dir.join("sda/device/vendor"), "ATA").unwrap();
        fs::write(dir.join("sda/device/model"), "KINGSTON SA400S3").unwrap();
        fs::write(dir.join("sda/size"), "2097152").unwrap();
        fs::create_dir_all(dir.join("sdb/device")).unwrap();
        fs::write(dir.join("sdb/device/vendor"), "Unknown").unwrap();
        let rows = disk_rows(&dir);
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(
            rows,
            vec![
                "/dev/sda: Kingston · 1.0 GiB",
                "/dev/sdb: Unknown · capacidade indisponível"
            ]
        );
    }
    #[test]
    fn sensor_readings_exclude_acpi() {
        let dir = std::env::temp_dir().join(format!(
            "reset-sensors-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        for (device, name, value, label) in [
            ("hwmon0", "acpitz", "27800", ""),
            ("hwmon1", "coretemp", "48000", "Core 0"),
        ] {
            let path = dir.join(device);
            fs::create_dir_all(&path).unwrap();
            fs::write(path.join("name"), name).unwrap();
            fs::write(path.join("temp1_input"), value).unwrap();
            fs::write(path.join("temp1_label"), label).unwrap();
        }
        let readings = sensor_readings(&dir);
        fs::remove_dir_all(dir).unwrap();
        assert_eq!(readings, vec![("coretemp".into(), "Core 0".into(), 48.0)]);
        assert_eq!(core_average(&readings), Some((48.0, 1)));
    }
    #[test]
    fn readable_sensor_names() {
        assert_eq!(sensor_label("coretemp", "Core 3"), "CPU · núcleo 3");
    }
    #[test]
    fn pci_names_do_not_confuse_other_vendors_or_subdevices() {
        let data = "10de  NVIDIA\n\t0152  Outra placa\n8086  Intel Corporation\n\t0152  Controlador gráfico\n\t\t1234 0001  Subdevice\n\t0153  Outra\n";
        assert_eq!(
            pci_names(data, "0x8086", "0x0152"),
            (
                Some("Intel Corporation".into()),
                Some("Controlador gráfico".into())
            )
        );
        assert_eq!(
            pci_names(data, "8086", "0001"),
            (Some("Intel Corporation".into()), None)
        );
        assert!(display_gpu("card1", "0x8086", "0x0152", "i915", data)
            .contains("Intel Corporation · Controlador gráfico"));
        assert!(display_gpu("card1", "0x8086", "0x9999", "i915", "")
            .contains("Intel · modelo não identificado (0x9999)"));
    }
    #[test]
    fn average_only_core_labels() {
        let values = [
            ("coretemp".into(), "Core 0".into(), 40.0),
            ("coretemp".into(), "Core 1".into(), 44.0),
            ("coretemp".into(), "Package id 0".into(), 90.0),
            ("other".into(), "Core 2".into(), 100.0),
        ];
        assert_eq!(core_average(&values), Some((42.0, 2)));
        assert_eq!(core_average(&[]), None);
    }
    #[test]
    fn report_escapes_untrusted_data_and_formats_time() {
        let report = report_html(
            "SISTEMA\nNome: <script>&\"",
            Some("ok & pronto"),
            0,
            ReportTheme::Light,
        );
        assert!(report.contains("&lt;script&gt;&amp;&quot;"));
        assert!(!report.contains("<script>"));
        assert!(report.contains("1970-01-01 00:00:00 UTC"));
        assert!(report.contains("ok &amp; pronto"));
    }
    #[test]
    fn report_uses_selected_theme() {
        let light = report_html("SISTEMA\nCPU: exemplo", None, 0, ReportTheme::Light);
        let dark = report_html("SISTEMA\nCPU: exemplo", None, 0, ReportTheme::Dark);
        for (report, scheme, background, card, text, accent) in [
            (&light, "light", "#f6f6f6", "#ffffff", "#403f53", "#365f9d"),
            (&dark, "dark", "#011627", "#0b253a", "#d6deeb", "#82aaff"),
        ] {
            assert!(report.contains(&format!("color-scheme:{scheme}")));
            assert!(report.contains(&format!("--background:{background}")));
            assert!(report.contains(&format!("--card:{card}")));
            assert!(report.contains(&format!("--text:{text}")));
            assert!(report.contains(&format!("--accent:{accent}")));
            assert!(report.contains("CPU: exemplo"));
            assert!(report.contains("RESULTADOS DA SESSÃO"));
        }
    }
    #[test]
    fn snapshot_has_sections() {
        let s = snapshot();
        assert!(s.contains("SISTEMA"));
        assert!(s.contains("SENSORES"));
    }
}
