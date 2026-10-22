#!/usr/bin/env python3
"""Served ground-truth sample for the browser (WebDriver) prefixed-view test.

Reads skt-morph-data (SKT_MORPH_DATA, default
/home/edhiraj/Documents/projs/pypanini/skt-morph-data), transliterates via
the audited CLI path, and writes dist/data/prefixed_truth_sample.json:
12 (dhātu, stack) combos covering the reported bug (01.1096+vi), facultative
stem variants (10.0343), sam-su sandhi (01.0025), all three pada classes,
karmaṇī, listed stacks (backfill path) and unlisted stacks (live-derive
path). Each combo carries `portal_stack` — the dropdown-offered name
(corpus `आङ्` vs portal `आ`; the engine treats them identically).

The selftest renders each combo in the real DOM and asserts every lakāra
section cell-exact. Absent corpus → file is not written; the browser test
skips the sweep with a warning instead of failing.
"""
import json
import os
import subprocess
import sys

COMBOS = [
    ("01.1096", "vi"), ("01.1096", "parA"), ("03.0010", "pra"), ("03.0010", "A"),
    ("10.0343", "A"), ("01.0025", "sam"), ("01.0001", "AN"), ("01.0001", "pra"),
    ("08.0010", "upa"), ("08.0010", "pari"), ("01.0007", "upa"), ("01.0005", "AN"),
]
FAMILIES = [
    ("shuddha", "ting", "yak"), ("nich", "nich", "nich_yak"),
    ("san", "san", "san_yak"), ("yang", "yang", "yang_yak"),
    ("yangluk", "yangluk", "yangluk_yak"),
]


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    root = os.path.join(here, "..")
    cor = os.environ.get(
        "SKT_MORPH_DATA", "/home/edhiraj/Documents/projs/pypanini/skt-morph-data"
    )
    if not os.path.isdir(cor):
        print(f"gen_truth_sample: no corpus at {cor}, skipping")
        return 0
    binary = os.path.join(root, "target", "release", "dhaturupa-wasm")
    if not os.path.isfile(binary):
        print("gen_truth_sample: no release binary (run cargo build --release), skipping")
        return 0
    data = json.load(open(os.path.join(root, "data", "dhatu_data.json")))
    singles = set()
    for entry in data["data"]:
        for u in entry.get("upasargas", []):
            if " + " not in u["name"]:
                singles.add(u["name"])

    cache = {}

    def run(*args):
        return subprocess.run(
            [binary, *args], capture_output=True, text=True, check=True
        ).stdout.strip()

    def deva(slp1):
        if slp1 not in cache:
            cache[slp1] = run("translit", "slp1", slp1)
        return cache[slp1]

    def norm_slp(deva_name):
        import re

        parts = []
        for tok in deva_name.split("+"):
            code = run("translit", "deva", tok.strip())
            parts.append(re.sub(r"N$", "", code))
        return ";".join(parts)

    singles_slp = {s: norm_slp(s) for s in singles}
    out = []
    for bid, stack in COMBOS:
        gana = bid.split(".")[0]
        with open(os.path.join(cor, gana, f"{bid}.json")) as f:
            entry = json.load(f)
        uf = entry["upasarga_forms"][stack]["conjugations"]
        stack_deva = " + ".join(deva(t) for t in stack.split(";"))
        want = ";".join(
            __import__("re").sub(r"N$", "", t) for t in stack.split(";")
        )
        listed = next(
            x for x in data["data"] if x["baseindex"] == bid
        ).get("upasargas", [])
        portal = next(
            (u["name"] for u in listed if norm_slp(u["name"]) == want), None
        )
        if portal is None:
            portal = next(
                (s for s, code in singles_slp.items() if code == want), stack_deva
            )
        item = {"baseindex": bid, "stack": stack_deva,
                "portal_stack": portal, "families": {}}
        for fam, map_kartari, map_karmani in FAMILIES:
            for prayoga, map_name in [("kartari", map_kartari),
                                      ("karmani", map_karmani)]:
                lakaras = {}
                for lakara, slots in uf.get(map_name, {}).items():
                    if slots:
                        lakaras[lakara] = [deva(s) for s in slots]
                if lakaras:
                    item["families"].setdefault(fam, {})[prayoga] = lakaras
        out.append(item)
    dest = os.path.join(root, "dist", "data", "prefixed_truth_sample.json")
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    json.dump(out, open(dest, "w"), ensure_ascii=False)
    print(f"gen_truth_sample: {len(out)} combos, "
          f"{len(cache)} unique forms, {os.path.getsize(dest)} bytes -> {dest}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
