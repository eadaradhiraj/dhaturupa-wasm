fn main() {
    for s in ["aSTAhRe", "aSTAhRi", "aSTAhni", "aSTAhR", "ahaGkAravad"] {
        let d = dhaturupa_wasm::translit::slp1_to_deva(s).unwrap();
        let cps: Vec<String> = d.chars().map(|c| format!("U+{:04X}", c as u32)).collect();
        println!("{s} -> {d}  [{}]", cps.join(" "));
    }
}
