//! Minimal CLI clone of `ashtadhyayi.com/dhatu` (list + info/upasarga/ting/krut + SLP1 join).

use dhaturupa_wasm::hk::hk_to_slp1;
use dhaturupa_wasm::revindex::{audit_all, AuditData, AuditOptions};
use dhaturupa_wasm::translit::{contains_deva, deva_stack_to_slp1, deva_to_slp1, slp1_to_deva};
use dhaturupa_wasm::{
    baseindex_map, find_by_baseindex, format_detail_info, format_list_row, format_upasarga,
    load_file, DhatuFilter,
};
use std::path::{Path, PathBuf};

fn print_usage() {
    eprintln!(
        "usage:\n\
         \tdhaturupa-wasm list [--data PATH] [--sort az] [--gana 1] [--pada P] [--settva S] [--karma A] [--limit N]\n\
         \tdhaturupa-wasm show <baseindex> [--data PATH]\n\
         \tdhaturupa-wasm upasarga <baseindex> [--data PATH]\n\
         \tdhaturupa-wasm ting <baseindex> [--family shuddha|nich|san|yang|yangluk] [--prayoga kartari|karmani] [--forms PATH]\n\
         \tdhaturupa-wasm krut <baseindex> [--family shuddha|nich|san|yang|yangluk] [--krut PATH]\n\
         \tdhaturupa-wasm search <form> [--family F] [--limit N]\n\
         \tdhaturupa-wasm audit [--full]\n\
         \tdhaturupa-wasm build-blobs --out DIR\n\
         \tdhaturupa-wasm build-site --out DIR\n\
         \tdhaturupa-wasm build-index --out DIR [--family F] [--shards N]\n\
         \tdhaturupa-wasm build-subanta-index --out DIR [--shards N]\n\
         \tdhaturupa-wasm subanta <stem> <m|f|n> [--slp1]         (SLP1 or Devanagari in, Devanagari out)\n\
         \tdhaturupa-wasm krttable <family> <krt> <m|f|n> <value> [--slp1]\n\
         \tdhaturupa-wasm prefix <stack> <base> [--slp1]         (e.g. 'वि + अति' भवति)\n\
         \tdhaturupa-wasm translit <text>                     (auto-detects script)\n\
         \tdhaturupa-wasm slp1 <baseindex> [--data PATH] [--vidyut PATH]\n\n\
         fetch data first (pinned a9f9112):\n\
         \tcurl -sL https://cdn.jsdelivr.net/gh/ashtadhyayi-com/data@a9f9112/dhatu/data.txt -o data/dhatu_data.json\n\
         \tcurl -sL https://cdn.jsdelivr.net/gh/ashtadhyayi-com/data@a9f9112/dhatu/vidyut/vidyut_dhatupatha_5.tsv -o data/vidyut_dhatupatha_5.tsv\n\
         \tfor fam in shuddha nich san yang yangluk; do \\\n\
         \t  curl -sL https://cdn.jsdelivr.net/gh/ashtadhyayi-com/data@a9f9112/dhatu/dhatuforms_vidyut_${{fam}}_kartari.txt -o data/ting_${{fam}}_kartari.json; \\\n\
         \t  curl -sL https://cdn.jsdelivr.net/gh/ashtadhyayi-com/data@a9f9112/dhatu/dhatuforms_vidyut_${{fam}}_karmani.txt -o data/ting_${{fam}}_karmani.json; \\\n\
         \t  curl -sL https://cdn.jsdelivr.net/gh/ashtadhyayi-com/data@a9f9112/dhatu/dhatuforms_vidyut_${{fam}}_krut.txt -o data/krut_${{fam}}.json; \\\n\
         \tdone"
    );
}

fn data_path(args: &[String]) -> PathBuf {
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--data" && i + 1 < args.len() {
            return PathBuf::from(&args[i + 1]);
        }
        i += 1;
    }
    PathBuf::from("data/dhatu_data.json")
}

fn opt_value(args: &[String], key: &str) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == key && i + 1 < args.len() {
            return Some(args[i + 1].clone());
        }
        i += 1;
    }
    None
}

fn load_or_exit(path: &Path) -> dhaturupa_wasm::DhatuFile {
    match load_file(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("cannot load {}: {e}", path.display());
            eprintln!("fetch with:");
            eprintln!("  curl -sL https://cdn.jsdelivr.net/gh/ashtadhyayi-com/data@a9f9112/dhatu/data.txt -o {}", path.display());
            std::process::exit(1);
        }
    }
}

fn cmd_list(args: &[String]) {
    let path = data_path(args);
    let file = load_or_exit(&path);
    let filter = DhatuFilter {
        ganas: opt_value(args, "--gana")
            .map(|v| vec![v])
            .unwrap_or_default(),
        padas: opt_value(args, "--pada")
            .map(|v| vec![v])
            .unwrap_or_default(),
        settvas: opt_value(args, "--settva")
            .map(|v| vec![v])
            .unwrap_or_default(),
        karmas: opt_value(args, "--karma")
            .map(|v| vec![v])
            .unwrap_or_default(),
    };
    let mut rows = filter.apply(&file.data);
    if opt_value(args, "--sort").as_deref() == Some("az") {
        rows.sort_by(|a, b| a.dhatu.cmp(&b.dhatu));
    }
    let limit: usize = opt_value(args, "--limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(usize::MAX);
    println!(
        "॥ अथ पाणिनीयधातुपाठः ॥ ({} of {})",
        rows.len().min(limit),
        file.data.len()
    );
    for e in rows.into_iter().take(limit) {
        println!("{}", format_list_row(e));
    }
}

fn cmd_show(args: &[String]) {
    let Some(id) = args.first().cloned() else {
        print_usage();
        std::process::exit(2);
    };
    let rest = if args.len() > 1 { &args[1..] } else { &[] };
    let file = load_or_exit(&data_path(rest));
    if let Some(e) = find_by_baseindex(&file, &id) {
        print!("{}", format_detail_info(e));
    } else {
        eprintln!("unknown dhatu {id} (try 01.0001, or 1.1)");
        std::process::exit(1);
    }
}

fn cmd_upasarga(args: &[String]) {
    let Some(id) = args.first().cloned() else {
        print_usage();
        std::process::exit(2);
    };
    let rest = if args.len() > 1 { &args[1..] } else { &[] };
    let file = load_or_exit(&data_path(rest));
    if let Some(e) = find_by_baseindex(&file, &id) {
        print!("{}", format_upasarga(e));
    } else {
        eprintln!("unknown dhatu {id}");
        std::process::exit(1);
    }
}

fn cmd_slp1(args: &[String]) {
    let Some(id) = args.first().cloned() else {
        print_usage();
        std::process::exit(2);
    };
    let rest = if args.len() > 1 { &args[1..] } else { &[] };
    let vpath =
        opt_value(rest, "--vidyut").unwrap_or_else(|| "data/vidyut_dhatupatha_5.tsv".to_string());
    let file = load_or_exit(&data_path(rest));
    let entry = find_by_baseindex(&file, &id).unwrap_or_else(|| {
        eprintln!("unknown dhatu {id}");
        std::process::exit(1);
    });
    let tsv = std::fs::read_to_string(&vpath).unwrap_or_else(|_| {
        eprintln!("cannot load {vpath}; fetch with:");
        eprintln!("  curl -sL https://cdn.jsdelivr.net/gh/ashtadhyayi-com/data@a9f9112/dhatu/vidyut/vidyut_dhatupatha_5.tsv -o {vpath}");
        std::process::exit(1);
    });
    let rows = dhaturupa_wasm::vidyut::load_tsv_str(&tsv);
    let _map = baseindex_map(&file);
    if let Some(r) = dhaturupa_wasm::vidyut::find(&rows, &entry.baseindex) {
        let stripped: String = r.dhatu.chars().filter(|c| *c != '~' && *c != ' ').collect();
        assert!(
            dhaturupa_wasm::slp1::is_slp1_form(&stripped) || r.dhatu.contains('~'),
            "vidyut dhatu should be SLP1-ish"
        );
        println!(
            "{} {} -> SLP1 {} ({})",
            entry.baseindex, entry.dhatu, r.dhatu, r.artha
        );
    } else {
        eprintln!("no vidyut row for {}", entry.baseindex);
        std::process::exit(1);
    }
}

fn cmd_ting(args: &[String]) {
    let Some(id) = args.first().cloned() else {
        print_usage();
        std::process::exit(2);
    };
    let rest = if args.len() > 1 { &args[1..] } else { &[] };
    let fpath = opt_value(rest, "--forms").unwrap_or_else(|| {
        let family = opt_value(rest, "--family").unwrap_or_else(|| "shuddha".to_string());
        let prayoga = opt_value(rest, "--prayoga").unwrap_or_else(|| "kartari".to_string());
        dhaturupa_wasm::forms::ting_filename(&family, &prayoga).unwrap_or_else(|| {
            eprintln!(
                "unknown family/prayoga '{family}/{prayoga}' (families: {:?}; prayogas: {:?})",
                dhaturupa_wasm::forms::TING_FAMILIES,
                dhaturupa_wasm::forms::TING_PRAYOGAS
            );
            std::process::exit(2);
        })
    });
    let text = std::fs::read_to_string(&fpath).unwrap_or_else(|_| {
        eprintln!("cannot load {fpath}");
        std::process::exit(1);
    });
    let map = dhaturupa_wasm::forms::parse_forms_map(&text).unwrap_or_else(|e| {
        eprintln!("bad forms json: {e}");
        std::process::exit(1);
    });
    let norm = dhaturupa_wasm::normalize_baseindex(&id);
    if let Some(entry) = map.get(&norm) {
        print!("{}", dhaturupa_wasm::forms::format_ting(entry));
    } else {
        eprintln!("no ting row for {norm}");
        std::process::exit(1);
    }
}

fn cmd_krut(args: &[String]) {
    let Some(id) = args.first().cloned() else {
        print_usage();
        std::process::exit(2);
    };
    let rest = if args.len() > 1 { &args[1..] } else { &[] };
    let fpath = opt_value(rest, "--krut").unwrap_or_else(|| {
        let family = opt_value(rest, "--family").unwrap_or_else(|| "shuddha".to_string());
        dhaturupa_wasm::forms::krut_filename(&family).unwrap_or_else(|| {
            eprintln!(
                "unknown family '{family}' (families: {:?})",
                dhaturupa_wasm::forms::KRUT_FAMILIES
            );
            std::process::exit(2);
        })
    });
    let text = std::fs::read_to_string(&fpath).unwrap_or_else(|_| {
        eprintln!("cannot load {fpath}");
        std::process::exit(1);
    });
    let map = dhaturupa_wasm::forms::parse_forms_map(&text).unwrap_or_else(|e| {
        eprintln!("bad krut json: {e}");
        std::process::exit(1);
    });
    let norm = dhaturupa_wasm::normalize_baseindex(&id);
    if let Some(entry) = map.get(&norm) {
        print!("{}", dhaturupa_wasm::forms::format_krut(entry));
    } else {
        eprintln!("no krut row for {norm}");
        std::process::exit(1);
    }
}

/// Devanagari input → SLP1; SLP1 passes through. Exits 2 on failure.
fn to_slp1(arg: &str, what: &str) -> String {
    if contains_deva(arg) {
        deva_to_slp1(arg).unwrap_or_else(|| {
            eprintln!("cannot transliterate {what} '{arg}' (separators/unknown chars?)");
            std::process::exit(2);
        })
    } else {
        arg.to_string()
    }
}

/// Display form: Devanagari by portal contract, raw SLP1 with `--slp1`.
/// Unconvertible output falls back to SLP1 (never fails silently).
fn show_form(form: &str, slp1: bool) -> String {
    if slp1 {
        form.to_string()
    } else {
        slp1_to_deva(form).unwrap_or_else(|| form.to_string())
    }
}

fn print_paradigm(table: &dhaturupa_wasm::engine::subanta::Paradigm, slp1: bool) {
    println!(
        "{} {} ({:?}):",
        show_form(&table.stem, slp1),
        table.linga,
        table.class
    );
    for cell in &table.cells {
        let forms: Vec<String> = cell.forms.iter().map(|f| show_form(f, slp1)).collect();
        println!("  {} {}: {}", cell.vibhakti, cell.vacana, forms.join(","));
    }
}

fn cmd_subanta(args: &[String]) {
    let (Some(stem_arg), Some(linga_s)) = (args.first().cloned(), args.get(1).cloned()) else {
        eprintln!("usage: dhaturupa-wasm subanta <stem> <m|f|n> [--slp1]  (e.g. subanta rAma m)");
        std::process::exit(2);
    };
    let slp1 = args.iter().any(|arg| arg == "--slp1");
    let stem = to_slp1(&stem_arg, "stem");
    let Some(linga) = dhaturupa_wasm::engine::subanta::Linga::parse(&linga_s) else {
        eprintln!("linga must be m, f or n");
        std::process::exit(2);
    };
    let tables = dhaturupa_wasm::engine::subanta::generate_tables(&stem, linga);
    if tables.is_empty() {
        println!("not possible");
        return;
    }
    for t in &tables {
        print_paradigm(t, slp1);
    }
}

fn cmd_krttable(args: &[String]) {
    if let (Some(family_arg), Some(krt_arg), Some(field), Some(value_arg)) =
        (args.first(), args.get(1), args.get(2), args.get(3))
    {
        let Some(family) = dhaturupa_wasm::forms::normalize_krut_engine_family(family_arg) else {
            eprintln!("unknown krut family '{family_arg}' (try shuddha|nich|san|yang|yangluk or krut|nich_krut|san_krut|yang_krut|yangluk_krut)");
            std::process::exit(2);
        };
        let krt = to_slp1(krt_arg, "krt");
        // `/`-alternant cells are per-alternant tables: try each until one works.
        let values: Vec<String> = if value_arg.contains('/') {
            value_arg.split('/').map(|a| to_slp1(a, "value")).collect()
        } else {
            vec![to_slp1(value_arg, "value")]
        };
        let mut shown = false;
        let slp1 = args.iter().any(|arg| arg == "--slp1");
        for value in &values {
            if let Some(t) = dhaturupa_wasm::engine::subanta::krt_table(family, &krt, field, value)
            {
                print_paradigm(&t, slp1);
                shown = true;
            }
        }
        if !shown {
            println!("not possible");
        }
    } else {
        eprintln!("usage: dhaturupa-wasm krttable <family> <krt> <m|f|n> <value> [--slp1]  (e.g. krttable krut Satf f BavantI)");
        std::process::exit(2);
    }
}

fn cmd_prefix(args: &[String]) {
    let (Some(stack_arg), Some(base_arg)) = (args.first().cloned(), args.get(1).cloned()) else {
        eprintln!("usage: dhaturupa-wasm prefix <stack> <base> [--slp1] [--family F] [--dhatu B]  (e.g. prefix pra Bavati)");
        std::process::exit(2);
    };
    let slp1 = args.iter().any(|arg| arg == "--slp1");
    let stack = if contains_deva(&stack_arg) {
        deva_stack_to_slp1(&stack_arg).unwrap_or_else(|| {
            eprintln!("cannot transliterate stack '{stack_arg}'");
            std::process::exit(2);
        })
    } else {
        stack_arg
    };
    let base = to_slp1(&base_arg, "base");
    if !dhaturupa_wasm::slp1::is_slp1_form(&base)
        || base.contains(';')
        || stack
            .split(';')
            .any(|p| !dhaturupa_wasm::slp1::is_slp1_form(p))
    {
        eprintln!("stack and base must be SLP1 letters (`;` separates stack)");
        std::process::exit(2);
    }
    // Context: `--family` selects the engine category (derived/intensive
    // gates); `--dhatu` adds the vidyut lexical flags. Defaults preserve the
    // old root-level behavior.
    let rest = if args.len() > 1 { &args[1..] } else { &[] };
    let family = opt_value(rest, "--family").unwrap_or_else(|| "shuddha".to_string());
    let category = dhaturupa_wasm::forms::engine_ting_category(&family).unwrap_or_else(|| {
        eprintln!("unknown family '{family}' (try shuddha|nich|san|yang|yangluk)");
        std::process::exit(2);
    });
    let ctx = if let Some(id) = opt_value(rest, "--dhatu") {
        let vpath = opt_value(rest, "--vidyut")
            .unwrap_or_else(|| "data/vidyut_dhatupatha_5.tsv".to_string());
        let text = std::fs::read_to_string(&vpath).unwrap_or_else(|_| {
            eprintln!("cannot load {vpath}");
            std::process::exit(1);
        });
        let rows = dhaturupa_wasm::vidyut::load_tsv_str(&text);
        let dhatu = dhaturupa_wasm::vidyut::find(&rows, &id).unwrap_or_else(|| {
            eprintln!("no vidyut row for {id}");
            std::process::exit(1);
        });
        let file = load_or_exit(&data_path(rest));
        let note = dhaturupa_wasm::find_by_baseindex(&file, &id)
            .map_or("", |entry| entry.dhatu_note.as_str());
        dhaturupa_wasm::genctx::ctx_for(&dhatu.dhatu, category, note)
    } else {
        dhaturupa_wasm::engine::upasarga::GenCtx {
            derived: dhaturupa_wasm::engine::upasarga::is_derived_category(category),
            intensive_keep: category.starts_with("yang"),
            yangluk: category.starts_with("yangluk"),
            ..dhaturupa_wasm::engine::upasarga::GenCtx::default()
        }
    };
    let out = dhaturupa_wasm::engine::upasarga::generate_prefixed(&stack, &base, ctx);
    for f in &out {
        println!("{}", show_form(f, slp1));
    }
}

/// Exact form search over one family's Devanagari tables.
///
/// Portal rule: Devanagari input is used as-is; ASCII input tries the union
/// of HK-decoded and raw-SLP1-decoded Devanagari (SLP1 stays hidden, so both
/// readings are searched). `/`-alternant krt cells match per alternant.
/// Prints `baseindex kind location` lines.
fn cmd_search(args: &[String]) {
    let Some(query_arg) = args.first().cloned() else {
        eprintln!("usage: dhaturupa-wasm search <form> [--family F] [--limit N]");
        std::process::exit(2);
    };
    let rest = if args.len() > 1 { &args[1..] } else { &[] };
    let family = opt_value(rest, "--family").unwrap_or_else(|| "shuddha".to_string());
    let limit: usize = opt_value(rest, "--limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(50);
    let queries: Vec<String> = if contains_deva(&query_arg) {
        vec![query_arg.clone()]
    } else {
        let mut out = Vec::new();
        if let Some(decoded) = hk_to_slp1(&query_arg).and_then(|slp| slp1_to_deva(&slp)) {
            out.push(decoded);
        }
        if let Some(deva) = slp1_to_deva(&query_arg) {
            if !out.contains(&deva) {
                out.push(deva);
            }
        }
        if out.is_empty() {
            eprintln!(
                "cannot transliterate query '{query_arg}' (need Devanagari or Harvard-Kyoto)"
            );
            std::process::exit(2);
        }
        out
    };
    let matches = |form: &str| queries.iter().any(|query| query == form);
    let mut hits: Vec<String> = Vec::new();
    // Ting: both prayogas for the family.
    for prayoga in ["kartari", "karmani"] {
        let Some(path) = dhaturupa_wasm::forms::ting_filename(&family, prayoga) else {
            continue;
        };
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(map) = dhaturupa_wasm::forms::parse_forms_map(&text) else {
            continue;
        };
        let mut ids: Vec<&String> = map.keys().collect();
        ids.sort();
        for id in ids {
            let mut lakaras: Vec<&String> = map[id].keys().collect();
            lakaras.sort();
            for lak in lakaras {
                for (pos, slot) in dhaturupa_wasm::forms::split_slots(&map[id][lak])
                    .iter()
                    .enumerate()
                {
                    if dhaturupa_wasm::forms::split_variants(slot)
                        .iter()
                        .any(|v| matches(v))
                    {
                        hits.push(format!("{id} ting {family}/{prayoga} {lak}[{pos}]"));
                    }
                }
            }
        }
    }
    if let Some(path) = dhaturupa_wasm::forms::krut_filename(&family) {
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Ok(map) = dhaturupa_wasm::forms::parse_forms_map(&text) {
                let mut ids: Vec<&String> = map.keys().collect();
                ids.sort();
                for id in ids {
                    let mut krts: Vec<&String> = map[id].keys().collect();
                    krts.sort();
                    for krt in krts {
                        for (row, cell) in dhaturupa_wasm::forms::parse_krt_rows(&map[id][krt])
                            .iter()
                            .enumerate()
                        {
                            for (field, value) in [
                                ("base", cell.base.as_str()),
                                ("m", cell.m.as_str()),
                                ("f", cell.f.as_str()),
                                ("n", cell.n.as_str()),
                            ] {
                                if value.is_empty() {
                                    continue;
                                }
                                if dhaturupa_wasm::slp1::split_alternants(value)
                                    .iter()
                                    .any(|a| matches(a))
                                {
                                    hits.push(format!("{id} krut {family} {krt}[{row}].{field}"));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if hits.is_empty() {
        println!("no hits for {query_arg} (family {family})");
    } else {
        println!("{} hit(s) for {query_arg} (family {family}):", hits.len());
        for h in hits.into_iter().take(limit) {
            println!("{h}");
        }
    }
}

/// Reverse-search audit: every displayable form must resolve back.
/// Sampled by default; `--full` checks everything (slow, run in release).
fn cmd_audit(args: &[String]) {
    let full = args.iter().any(|arg| arg == "--full");
    let opts = if full {
        AuditOptions::full()
    } else {
        AuditOptions::sampled()
    };
    let (ting, krut) = load_family_tables();
    let dhatu = load_or_exit(&crate::data_path(&[]));
    let vtext = std::fs::read_to_string("data/vidyut_dhatupatha_5.tsv").unwrap_or_else(|_| {
        eprintln!("cannot load data/vidyut_dhatupatha_5.tsv");
        std::process::exit(1);
    });
    let vidyut = dhaturupa_wasm::vidyut::load_tsv_str(&vtext)
        .into_iter()
        .map(|row| (row.code, row.dhatu))
        .collect();
    let data = AuditData {
        ting,
        krut,
        dhatu,
        vidyut,
    };
    let start = std::time::Instant::now();
    let report = audit_all(&data, &opts);
    println!(
        "audit ({}): checked={} misses={} elapsed={:.1}s",
        if full { "full" } else { "sampled" },
        report.checked,
        report.misses,
        start.elapsed().as_secs_f64()
    );
    println!(
        "  indexed: ting={} krt={} prefixed={} subanta={}",
        report.stats.ting_forms,
        report.stats.krt_forms,
        report.stats.prefixed_forms,
        report.stats.subanta_forms
    );
    println!(
        "  skipped: deva_in={} slp1_out={} table={} no_base={}",
        report.stats.skip_deva,
        report.stats.skip_slp1,
        report.stats.skip_table,
        report.stats.skip_no_base
    );
    for sample in &report.miss_samples {
        println!("  MISS: {sample}");
    }
    for sample in report.stats.skip_samples.iter().take(10) {
        println!("  SKIP: {sample}");
    }
    if report.misses > 0 {
        std::process::exit(1);
    }
}

/// Loaded family tables shared by `build-blobs` and `build-site`.
type FamilyTables = (
    Vec<(String, String, dhaturupa_wasm::forms::TingMap)>,
    Vec<(String, dhaturupa_wasm::forms::KrutMap)>,
);

/// Load all 15 family tables (exits 1 on any failure).
fn load_family_tables() -> FamilyTables {
    let mut ting = Vec::new();
    for family in dhaturupa_wasm::forms::TING_FAMILIES {
        for prayoga in dhaturupa_wasm::forms::TING_PRAYOGAS {
            let path =
                dhaturupa_wasm::forms::ting_filename(family, prayoga).expect("known ting family");
            let text = std::fs::read_to_string(&path).unwrap_or_else(|_| {
                eprintln!("cannot load {path}");
                std::process::exit(1);
            });
            let map = dhaturupa_wasm::forms::parse_forms_map(&text).unwrap_or_else(|err| {
                eprintln!("bad ting json {path}: {err}");
                std::process::exit(1);
            });
            ting.push((family.to_string(), prayoga.to_string(), map));
        }
    }
    let mut krut = Vec::new();
    for family in dhaturupa_wasm::forms::KRUT_FAMILIES {
        let path = dhaturupa_wasm::forms::krut_filename(family).expect("known krut family");
        let text = std::fs::read_to_string(&path).unwrap_or_else(|_| {
            eprintln!("cannot load {path}");
            std::process::exit(1);
        });
        let map = dhaturupa_wasm::forms::parse_forms_map(&text).unwrap_or_else(|err| {
            eprintln!("bad krut json {path}: {err}");
            std::process::exit(1);
        });
        krut.push((family.to_string(), map));
    }
    (ting, krut)
}

/// Load the vidyut `baseindex -> SLP1 dhatu` join (exits 1 on failure).
fn load_vidyut_map(args: &[String]) -> std::collections::HashMap<String, String> {
    let vpath =
        opt_value(args, "--vidyut").unwrap_or_else(|| "data/vidyut_dhatupatha_5.tsv".to_string());
    let text = std::fs::read_to_string(&vpath).unwrap_or_else(|_| {
        eprintln!("cannot load {vpath}");
        std::process::exit(1);
    });
    dhaturupa_wasm::vidyut::load_tsv_str(&text)
        .into_iter()
        .map(|row| (row.code, row.dhatu))
        .collect()
}

/// Load the chandrika `baseindex -> [(upasarga, entry)]` join (empty map
/// when the file is absent — blobs simply carry no chandrika section).
fn load_chandrika_map(
    args: &[String],
) -> std::collections::HashMap<String, Vec<(String, dhaturupa_wasm::chandrika::ChandrikaEntry)>> {
    let cpath = opt_value(args, "--chandrika")
        .unwrap_or_else(|| "data/upasargartha_chandrika.json".to_string());
    let Ok(text) = std::fs::read_to_string(&cpath) else {
        return std::collections::HashMap::new();
    };
    match dhaturupa_wasm::chandrika::load_str(&text) {
        Ok(file) => dhaturupa_wasm::chandrika::baseindex_map(&file)
            .into_iter()
            .map(|(baseindex, entries)| {
                (
                    baseindex.to_string(),
                    entries
                        .into_iter()
                        .map(|(name, entry)| (name.to_string(), entry.clone()))
                        .collect(),
                )
            })
            .collect(),
        Err(_) => std::collections::HashMap::new(),
    }
}

/// Build one compact portal JSON per dhatu (`<out>/<baseindex>.json`).
fn cmd_build_blobs(args: &[String]) {
    let Some(out) = opt_value(args, "--out") else {
        eprintln!("usage: dhaturupa-wasm build-blobs --out DIR");
        std::process::exit(2);
    };
    let start = std::time::Instant::now();
    let dhatu = load_or_exit(&data_path(args));
    let (ting, krut) = load_family_tables();
    // Global singles mirror the portal dropdown (see blobs::build_blob).
    let mut singles: Vec<String> = Vec::new();
    for entry in &dhatu.data {
        for u in &entry.upasargas {
            if !u.name.contains(" + ") && !singles.contains(&u.name) {
                singles.push(u.name.clone());
            }
        }
    }
    singles.sort();
    let ting_refs: Vec<(String, String, &dhaturupa_wasm::forms::TingMap)> = ting
        .iter()
        .map(|(family, prayoga, map)| (family.clone(), prayoga.clone(), map))
        .collect();
    let krut_refs: Vec<(String, &dhaturupa_wasm::forms::KrutMap)> = krut
        .iter()
        .map(|(family, map)| (family.clone(), map))
        .collect();
    let sources = dhaturupa_wasm::blobs::BlobSources {
        ting: ting_refs,
        krut: krut_refs,
        vidyut: load_vidyut_map(args),
        chandrika: load_chandrika_map(args),
        singles,
        morphdata_root: dhaturupa_wasm::morphdata::corpus_root(),
        backfill_stats: std::cell::RefCell::new(dhaturupa_wasm::morphdata::BackfillStats::default()),
    };
    let (blobs, skipped) = dhaturupa_wasm::blobs::build_all(&dhatu, &sources);
    if let Err(err) = std::fs::create_dir_all(&out) {
        eprintln!("cannot create {out}: {err}");
        std::process::exit(1);
    }
    let mut bytes = 0_u64;
    for blob in &blobs {
        let path = std::path::Path::new(&out).join(format!("{}.json", blob.baseindex));
        let text = blob.payload.to_string();
        bytes += text.len() as u64;
        if let Err(err) = std::fs::write(&path, text) {
            eprintln!("cannot write {}: {err}", path.display());
            std::process::exit(1);
        }
    }
    println!(
        "blobs: built={} skipped={} bytes={} elapsed={:.1}s -> {out}/<baseindex>.json",
        blobs.len(),
        skipped,
        bytes,
        start.elapsed().as_secs_f64()
    );
    let stats = sources.backfill_stats.borrow();
    if sources.morphdata_root.is_none() {
        eprintln!(
            "blobs: no skt-morph-data corpus (SKT_MORPH_DATA) — ting_prefixed backfill skipped"
        );
    } else {
        eprintln!(
            "blobs: ting_prefixed backfill lakaras={} from_corpus={} engine_fallback={} missing={}",
            stats.lakaras, stats.from_corpus, stats.engine_fallback, stats.missing
        );
    }
}

/// Assemble the static portal: web shell + data + blobs under `<out>/`.
/// `pkg/` (wasm-pack output) is not built here; see README.
fn cmd_build_site(args: &[String]) {
    let Some(out) = opt_value(args, "--out") else {
        eprintln!("usage: dhaturupa-wasm build-site --out DIR");
        std::process::exit(2);
    };
    let root = std::path::Path::new(&out);
    let data = root.join("data");
    let shards = data.join("shards");
    let blobs = data.join("blobs");
    for dir in [root, &data, &shards, &blobs] {
        if let Err(err) = std::fs::create_dir_all(dir) {
            eprintln!("cannot create {}: {err}", dir.display());
            std::process::exit(1);
        }
    }
    for page in [
        ("web/app.html", "index.html"),
        ("web/app.js", "app.js"),
        ("web/render.js", "render.js"),
        ("web/sw.js", "sw.js"),
    ] {
        let dest = root.join(page.1);
        if let Err(err) = std::fs::copy(page.0, &dest) {
            eprintln!("cannot copy {} -> {}: {err}", page.0, dest.display());
            std::process::exit(1);
        }
    }
    let dhatu_src = data_path(args);
    if let Err(err) = std::fs::copy(&dhatu_src, data.join("dhatu_data.json")) {
        eprintln!("cannot copy {}: {err}", dhatu_src.display());
        std::process::exit(1);
    }
    let (ting, krut) = load_family_tables();
    let dhatu_site = load_or_exit(&data_path(args));
    let vidyut_site = load_vidyut_map(args);
    // Hash-sharded search sets (one shard of each answers any query):
    // base tinanta/krdanta forms (with per-dhatu prefixed surfaces folded
    // in) plus global subanta (declined) forms.
    let merged = build_merged_with_prefixed(&ting, &krut, &dhatu_site, &vidyut_site, None);
    for (shard, mut bucket) in
        dhaturupa_wasm::revindex::partition(merged, dhaturupa_wasm::revindex::PORTAL_SHARDS)
            .into_iter()
            .enumerate()
    {
        let text = dhaturupa_wasm::revindex::to_compact_value(&mut bucket).to_string();
        let dest = shards.join(format!("shard-{shard:02}.index.json"));
        if let Err(err) = std::fs::write(&dest, &text) {
            eprintln!("cannot write {}: {err}", dest.display());
            std::process::exit(1);
        }
    }
    let subanta = build_subanta_merged(&krut);
    for (shard, mut bucket) in
        dhaturupa_wasm::revindex::partition(subanta, dhaturupa_wasm::revindex::PORTAL_SHARDS)
            .into_iter()
            .enumerate()
    {
        let text = dhaturupa_wasm::revindex::to_compact_value(&mut bucket).to_string();
        let dest = shards.join(format!("sub-{shard:02}.index.json"));
        if let Err(err) = std::fs::write(&dest, &text) {
            eprintln!("cannot write {}: {err}", dest.display());
            std::process::exit(1);
        }
    }
    let dhatu = load_or_exit(&data_path(args));
    let mut singles: Vec<String> = Vec::new();
    for entry in &dhatu.data {
        for u in &entry.upasargas {
            if !u.name.contains(" + ") && !singles.contains(&u.name) {
                singles.push(u.name.clone());
            }
        }
    }
    singles.sort();
    let ting_refs: Vec<(String, String, &dhaturupa_wasm::forms::TingMap)> = ting
        .iter()
        .map(|(family, prayoga, map)| (family.clone(), prayoga.clone(), map))
        .collect();
    let krut_refs: Vec<(String, &dhaturupa_wasm::forms::KrutMap)> = krut
        .iter()
        .map(|(family, map)| (family.clone(), map))
        .collect();
    let sources = dhaturupa_wasm::blobs::BlobSources {
        ting: ting_refs,
        krut: krut_refs,
        vidyut: load_vidyut_map(args),
        chandrika: load_chandrika_map(args),
        singles,
        morphdata_root: dhaturupa_wasm::morphdata::corpus_root(),
        backfill_stats: std::cell::RefCell::new(dhaturupa_wasm::morphdata::BackfillStats::default()),
    };
    let (built, _) = dhaturupa_wasm::blobs::build_all(&dhatu, &sources);
    for blob in &built {
        let dest = blobs.join(format!("{}.json", blob.baseindex));
        if let Err(err) = std::fs::write(&dest, blob.payload.to_string()) {
            eprintln!("cannot write {}: {err}", dest.display());
            std::process::exit(1);
        }
    }
    println!(
        "site: blobs={} -> {} (then: wasm-pack build --target web --out-dir {out}/pkg; serve {out}/)",
        built.len(),
        blobs.display(),
    );
}

/// Merged base index over selected families, plus global prefixed surfaces
/// for every dhatu with upasarga stacks (per-form `GenCtx`). Powers the portal
/// shards: one shard answers base + prefixed queries across families.
fn build_merged_with_prefixed(
    ting: &[(String, String, dhaturupa_wasm::forms::TingMap)],
    krut: &[(String, dhaturupa_wasm::forms::KrutMap)],
    dhatu: &dhaturupa_wasm::DhatuFile,
    vidyut: &std::collections::HashMap<String, String>,
    only: Option<&str>,
) -> dhaturupa_wasm::revindex::RevIndex {
    use dhaturupa_wasm::revindex::LocKind;
    let mut merged = dhaturupa_wasm::revindex::RevIndex::default();
    let mut stats = dhaturupa_wasm::revindex::Stats::default();
    let mut per_dhatu: dhaturupa_wasm::revindex::DhatuForms = std::collections::HashMap::new();
    for (fam, prayoga, map) in ting {
        if let Some(only) = only {
            if only != fam {
                continue;
            }
        }
        dhaturupa_wasm::revindex::index_ting_map(
            &mut merged,
            map,
            fam,
            prayoga,
            Some(&mut per_dhatu),
            &mut stats,
        );
    }
    for (fam, map) in krut {
        if let Some(only) = only {
            if only != fam {
                continue;
            }
        }
        dhaturupa_wasm::revindex::index_krut_map(
            &mut merged,
            map,
            fam,
            Some(&mut per_dhatu),
            &mut stats,
        );
    }
    for entry in &dhatu.data {
        if entry.upasargas.is_empty() {
            continue;
        }
        let Some(forms) = per_dhatu.get(&entry.baseindex) else {
            stats.skip_no_base += 1;
            continue;
        };
        let dhatu_slp = vidyut.get(&entry.baseindex).map(String::as_str);
        let ctx_for = |inner: &LocKind| {
            dhaturupa_wasm::genctx::ctx_for_loc(dhatu_slp, &entry.dhatu_note, inner)
        };
        for stack in entry.upasargas.iter().map(|item| &item.name) {
            dhaturupa_wasm::revindex::index_prefixed_dhatu(
                &mut merged,
                &entry.baseindex,
                stack,
                forms,
                &ctx_for,
                &mut stats,
            );
        }
        // Prefixed declined forms: decline each krt cell, then prefix every
        // paradigm surface (dual/plural prefixed forms live here).
        let mut declined: Vec<(LocKind, String)> = Vec::new();
        for (loc, alternant) in forms {
            if !matches!(loc, LocKind::Krt { .. }) {
                continue;
            }
            let LocKind::Krt {
                family,
                krt,
                row,
                field,
            } = loc
            else {
                continue;
            };
            if field != "m" && field != "f" && field != "n" {
                continue;
            }
            declined.extend(dhaturupa_wasm::revindex::subanta_pairs(
                &entry.baseindex,
                family,
                krt,
                *row,
                field,
                alternant,
                &mut stats,
            ));
        }
        for stack in entry.upasargas.iter().map(|item| &item.name) {
            dhaturupa_wasm::revindex::index_prefixed_dhatu(
                &mut merged,
                &entry.baseindex,
                stack,
                &declined,
                &ctx_for,
                &mut stats,
            );
        }
    }
    eprintln!(
        "prefixed: surfaces={} skip_deva={} skip_slp1={}",
        stats.prefixed_forms, stats.skip_deva, stats.skip_slp1
    );
    merged.finish();
    merged
}

/// Base reverse index for one family (both prayogas + krut), for artifacts.
fn build_family_index(
    ting: &[(String, String, dhaturupa_wasm::forms::TingMap)],
    krut: &[(String, dhaturupa_wasm::forms::KrutMap)],
    family: &str,
) -> dhaturupa_wasm::revindex::RevIndex {
    let mut index = dhaturupa_wasm::revindex::RevIndex::default();
    let mut stats = dhaturupa_wasm::revindex::Stats::default();
    for (fam, prayoga, map) in ting {
        if fam != family {
            continue;
        }
        dhaturupa_wasm::revindex::index_ting_map(
            &mut index,
            map,
            fam,
            prayoga,
            None::<&mut dhaturupa_wasm::revindex::DhatuForms>,
            &mut stats,
        );
    }
    for (fam, map) in krut {
        if fam != family {
            continue;
        }
        dhaturupa_wasm::revindex::index_krut_map(
            &mut index,
            map,
            fam,
            None::<&mut dhaturupa_wasm::revindex::DhatuForms>,
            &mut stats,
        );
    }
    index
}

/// Build shippable compact base indexes, one per family
/// (`<out>/<family>.index.json`). The portal fetches these instead of the
/// raw table files + runtime build.
fn cmd_build_index(args: &[String]) {
    let Some(out) = opt_value(args, "--out") else {
        eprintln!("usage: dhaturupa-wasm build-index --out DIR [--family F] [--shards N]");
        std::process::exit(2);
    };
    let only = opt_value(args, "--family");
    if let Some(family) = &only {
        if !dhaturupa_wasm::forms::KRUT_FAMILIES.contains(&family.as_str()) {
            eprintln!("unknown family '{family}' (try shuddha|nich|san|yang|yangluk)");
            std::process::exit(2);
        }
    }
    let shards: u32 = opt_value(args, "--shards")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let start = std::time::Instant::now();
    let (ting, krut) = load_family_tables();
    if let Err(err) = std::fs::create_dir_all(&out) {
        eprintln!("cannot create {out}: {err}");
        std::process::exit(1);
    }
    for family in dhaturupa_wasm::forms::KRUT_FAMILIES {
        if let Some(only) = &only {
            if only != family {
                continue;
            }
        }
        if shards > 0 {
            continue;
        }
        let mut index = build_family_index(&ting, &krut, family);
        let text = dhaturupa_wasm::revindex::to_compact_value(&mut index).to_string();
        let dest = std::path::Path::new(&out).join(format!("{family}.index.json"));
        if let Err(err) = std::fs::write(&dest, &text) {
            eprintln!("cannot write {}: {err}", dest.display());
            std::process::exit(1);
        }
        println!(
            "index {family}: forms={} postings={} bytes={} -> {}",
            index.len(),
            index.postings(),
            text.len(),
            dest.display()
        );
    }
    if shards > 0 {
        // Merged sharded set: base forms plus per-dhatu prefixed surfaces.
        let dhatu_all = load_or_exit(&data_path(args));
        let vidyut_all = load_vidyut_map(args);
        let merged =
            build_merged_with_prefixed(&ting, &krut, &dhatu_all, &vidyut_all, only.as_deref());
        let buckets = dhaturupa_wasm::revindex::partition(merged, shards);
        let (forms, postings) = buckets.iter().fold((0, 0), |(f, p), bucket| {
            (f + bucket.len(), p + bucket.postings())
        });
        for (shard, mut bucket) in buckets.into_iter().enumerate() {
            let text = dhaturupa_wasm::revindex::to_compact_value(&mut bucket).to_string();
            let dest = std::path::Path::new(&out).join(format!("shard-{shard:02}.index.json"));
            if let Err(err) = std::fs::write(&dest, &text) {
                eprintln!("cannot write {}: {err}", dest.display());
                std::process::exit(1);
            }
        }
        println!("shards={shards}: forms={forms} postings={postings} -> {out}/shard-*.index.json");
    }
    println!("elapsed={:.1}s", start.elapsed().as_secs_f64());
}

/// Merged global subanta index over all krut families (for sharding).
fn build_subanta_merged(
    krut: &[(String, dhaturupa_wasm::forms::KrutMap)],
) -> dhaturupa_wasm::revindex::RevIndex {
    let mut merged = dhaturupa_wasm::revindex::RevIndex::default();
    let mut stats = dhaturupa_wasm::revindex::Stats::default();
    for (family, map) in krut {
        let mut ids: Vec<&String> = map.keys().collect();
        ids.sort();
        for id in ids {
            let mut krts: Vec<&String> = map[id].keys().collect();
            krts.sort();
            for krt in krts {
                for (row, cell) in dhaturupa_wasm::forms::parse_krt_rows(&map[id][krt])
                    .iter()
                    .enumerate()
                {
                    for (field, value) in [("m", &cell.m), ("f", &cell.f), ("n", &cell.n)] {
                        if value.is_empty() {
                            continue;
                        }
                        dhaturupa_wasm::revindex::index_subanta_cell(
                            &mut merged,
                            &dhaturupa_wasm::revindex::SubantaCell {
                                baseindex: id,
                                family,
                                krt_deva: krt,
                                row,
                                field,
                                value_deva: value,
                            },
                            &mut stats,
                        );
                    }
                }
            }
        }
    }
    merged.finish();
    eprintln!(
        "subanta merged: subanta={} skip_table={}",
        stats.subanta_forms, stats.skip_table
    );
    merged
}

/// Build the global subanta set (`<out>/sub-NN.index.json`): every
/// declinable krt cell across families, declined and hash-sharded.
/// The portal queries one base shard + one subanta shard per reading.
fn cmd_build_subanta_index(args: &[String]) {
    let Some(out) = opt_value(args, "--out") else {
        eprintln!("usage: dhaturupa-wasm build-subanta-index --out DIR [--shards N]");
        std::process::exit(2);
    };
    let shards: u32 = opt_value(args, "--shards")
        .and_then(|v| v.parse().ok())
        .unwrap_or(dhaturupa_wasm::revindex::PORTAL_SHARDS);
    let start = std::time::Instant::now();
    let (_, krut) = load_family_tables();
    if let Err(err) = std::fs::create_dir_all(&out) {
        eprintln!("cannot create {out}: {err}");
        std::process::exit(1);
    }
    let merged = build_subanta_merged(&krut);
    let postings = merged.postings();
    for (shard, mut bucket) in dhaturupa_wasm::revindex::partition(merged, shards)
        .into_iter()
        .enumerate()
    {
        let text = dhaturupa_wasm::revindex::to_compact_value(&mut bucket).to_string();
        let dest = std::path::Path::new(&out).join(format!("sub-{shard:02}.index.json"));
        if let Err(err) = std::fs::write(&dest, &text) {
            eprintln!("cannot write {}: {err}", dest.display());
            std::process::exit(1);
        }
    }
    println!(
        "subanta shards={shards}: postings={postings} elapsed={:.1}s -> {out}/sub-*.index.json",
        start.elapsed().as_secs_f64()
    );
}

fn cmd_translit(args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: dhaturupa-wasm translit [hk|slp1|deva] <text>");
        std::process::exit(2);
    }
    // Explicit directions: `translit hk bhavati` -> Bavati.
    if args.len() >= 2 && matches!(args[0].as_str(), "hk" | "slp1" | "deva") {
        let text = args[1..].join(" ");
        let out = match args[0].as_str() {
            "hk" => hk_to_slp1(&text),
            "slp1" => slp1_to_deva(&text),
            _ => {
                if text.contains('+') || text.contains(';') {
                    deva_stack_to_slp1(&text)
                } else {
                    deva_to_slp1(&text)
                }
            }
        };
        if let Some(converted) = out {
            println!("{converted}");
        } else {
            eprintln!("cannot transliterate '{text}' as {}", args[0]);
            std::process::exit(1);
        }
        return;
    }
    let text = args.join(" ");
    if contains_deva(&text) {
        if text.contains('+') || text.contains(';') {
            if let Some(converted) = deva_stack_to_slp1(&text) {
                println!("{converted}");
            } else {
                eprintln!("cannot transliterate stack '{text}'");
                std::process::exit(1);
            }
        } else if let Some(converted) = deva_to_slp1(&text) {
            println!("{converted}");
        } else {
            eprintln!("cannot transliterate '{text}'");
            std::process::exit(1);
        }
    } else if let Some(converted) = slp1_to_deva(&text) {
        println!("{converted}");
        // Same letters, two readings: warn when HK decodes differently.
        if let Some(hk_slp) = hk_to_slp1(&text) {
            if hk_slp != text {
                eprintln!(
                    "note: also valid Harvard-Kyoto (-> SLP1 {hk_slp}); `translit hk` decodes it"
                );
            }
        }
    } else {
        eprintln!("cannot transliterate '{text}'");
        std::process::exit(1);
    }
}

fn main() {
    let all: Vec<String> = std::env::args().skip(1).collect();
    let Some((cmd, rest)) = all.split_first() else {
        print_usage();
        return;
    };
    match cmd.as_str() {
        "list" => cmd_list(rest),
        "show" => cmd_show(rest),
        "upasarga" => cmd_upasarga(rest),
        "ting" => cmd_ting(rest),
        "krut" => cmd_krut(rest),
        "subanta" => cmd_subanta(rest),
        "krttable" => cmd_krttable(rest),
        "prefix" => cmd_prefix(rest),
        "search" => cmd_search(rest),
        "audit" => cmd_audit(rest),
        "build-blobs" => cmd_build_blobs(rest),
        "build-site" => cmd_build_site(rest),
        "build-index" => cmd_build_index(rest),
        "build-subanta-index" => cmd_build_subanta_index(rest),
        "translit" => cmd_translit(rest),
        "slp1" => cmd_slp1(rest),
        _ => print_usage(),
    }
}
