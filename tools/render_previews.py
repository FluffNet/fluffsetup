"""Render actual QML pages safely and collect their text-bound checks."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import subprocess
import sys

parser = argparse.ArgumentParser()
parser.add_argument("--output", type=Path, required=True)
parser.add_argument("--binary", type=Path, default=Path("target/debug/fluffsetup"))
parser.add_argument("--logo", type=Path)
parser.add_argument("--icons", type=Path)
parser.add_argument("--keyboard-registry", type=Path)
parser.add_argument("--keyboard-translations", type=Path)
parser.add_argument("--keyboard-only", action="store_true")
parser.add_argument("--country-only", action="store_true")
parser.add_argument("--region")
parser.add_argument("--keyboards")
parser.add_argument("--timezone")
parser.add_argument("--timezone-search", help="Seed a safe timezone search preview")
parser.add_argument("--clock-sync", choices=["offline", "confirmed", "system"], help="Read real system sync status or explicitly simulate an offline/confirmed preview")
parser.add_argument("--languages", nargs="+")
parser.add_argument("--pages", nargs="+", type=int)
parser.add_argument("--sizes", nargs="+", help="Restrict captures to WIDTHxHEIGHT logical screen sizes")
parser.add_argument("--base-only", action="store_true", help="Use base/script choices without country-locale variants")
parser.add_argument("--cold-start", action="store_true", help="Capture initial selections without scripted interactions first")
parser.add_argument("--workers", type=int, default=8, choices=range(1, 9), help="Bound parallel previews on smaller test machines")
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=True)
binary = args.binary.resolve()
languages = [line.split("\t")[0] for line in Path("translations/languages.tsv").read_text().splitlines()[1:]]
region_rows = [line.split("\t") for line in Path("translations/regions.tsv").read_text().splitlines()[1:]]
regions = [row[1].split(".")[0] for row in region_rows]
regional_languages = sorted({row[0] for row in region_rows})
states = [(0, "language"), (1, "welcome"), (2, "system"), (3, "account"), (4, "applying"), (5, "ready"), (4, "error"), (7, "keyboard"), (8, "country"), (9, "timezone")]
hostname_examples = {
    "hostname-spaces": ("my laptop", 1),
    "hostname-symbol": ("fluff@pc", 1),
    "hostname-multiple": ("-my laptop@.", 4),
    "hostname-five-errors": ("-" + "a" * 252 + " @.", 5),
    "hostname-long": ("a" * 256, 1),
    "hostname-valid": ("fluff-pc.example", 0),
    "hostname-non-latin": ("א" * 100, 1),
}
states.extend((2, state) for state in hostname_examples)
sizes = [(2560, 1440), (1920, 1080), (1280, 720), (1024, 768), (800, 600), (720, 1280), (1080, 1920), (1920, 1200), (3440, 1440)]
jobs = {(language, 800, 600, page, state) for language in languages for page, state in states}
jobs.update((language, width, height, page, state)
            for language in ["he", "en", "de", "ar", "fa", "ur", "ps", "zh_Hans", "zh_Hant", "mn"]
            for width, height in sizes for page, state in states)
jobs.update((language, width, height, page, state)
            for language in ["kok", "lb", "mi", "quz", "gd", "tt", "ug", "ca_valencia", "cy", "sr_Latn", "pa_Arab"]
            for width, height in sizes for page, state in states)
jobs.update((region, 800, 600, page, state) for region in regions
            for page, state in states + [(6, "region")])
jobs.update((language, width, height, 6, "region")
            for language in regional_languages
            for width, height in sizes)
if args.languages:
    # An explicit language set must cover the requested grid, not just filter
    # the default representative-size sample (which is compact-only for some).
    jobs.update((language, width, height, page, state)
                for language in args.languages for width, height in sizes
                for page, state in states + ([(6, "region")] if language in regional_languages or language in regions else []))
if args.keyboard_only:
    jobs = {job for job in jobs if job[3] == 7}
    jobs.update((language, width, height, 7, "keyboard")
                for language in ["fr", "fr_BE", "fr_CA", "fr_CH", "de_AT", "de_CH"]
                for width, height in sizes)
    jobs.update((language, width, height, 7, state)
                for language in ["he", "en", "de", "fr"]
                for width, height in sizes
                for state in ["keyboard-four", "keyboard-search", "keyboard-empty"])
if args.languages:
    jobs = {job for job in jobs if job[0] in args.languages}
if args.country_only:
    jobs = {job for job in jobs if job[3] == 8}
if args.pages:
    jobs = {job for job in jobs if job[3] in args.pages}
if args.sizes:
    selected_sizes = {tuple(map(int, size.split("x"))) for size in args.sizes}
    jobs = {job for job in jobs if (job[1], job[2]) in selected_sizes}
if args.base_only:
    jobs = {job for job in jobs if job[0] in languages}

def render(job):
    language, width, height, page, state = job
    name = f"{language}-{width}x{height}-{state}"
    snapshot = args.output.resolve() / f"{name}.png"
    command = [str(binary), "--preview", f"--page={page}", f"--width={width}",
               f"--height={height}", f"--snapshot={snapshot}"]
    if state == "error":
        command.append("--error=1")
    if state in ["language", "keyboard", "country", "timezone"] and not args.cold_start:
        command.append("--exercise=1")
    if args.keyboard_registry:
        command.append(f"--keyboard-registry={args.keyboard_registry.resolve()}")
    if args.keyboard_translations:
        command.append(f"--keyboard-translations={args.keyboard_translations.resolve()}")
    if args.region:
        command.append(f"--region={args.region}")
    if args.keyboards:
        command.append(f"--keyboards={args.keyboards}")
    if args.timezone:
        command.append(f"--timezone={args.timezone}")
    if args.timezone_search is not None and page == 9:
        command.append(f"--timezone-search={args.timezone_search}")
    if args.clock_sync:
        command.append(f"--clock-sync={args.clock_sync}")
    if state == "keyboard-four":
        command.append("--keyboards=us,il,fr:oss,de:neo")
    if state == "keyboard-search":
        command.append("--keyboard-search=French")
    if state == "keyboard-empty":
        command.append("--keyboard-search=zzzzzzzzzz")
    if state in hostname_examples:
        command.append(f"--hostname={hostname_examples[state][0]}")
    if args.logo:
        command.append(f"--logo={args.logo.resolve().as_uri()}")
    if args.icons:
        command.append(f"--icons={args.icons.resolve().as_uri()}")
    if sys.platform == "darwin":
        fonts = {"hi": "Devanagari Sangam MN", "mr": "Devanagari Sangam MN", "ne": "Devanagari Sangam MN", "kok": "Devanagari Sangam MN",
                 "gu": "Gujarati Sangam MN", "ta": "Tamil Sangam MN", "te": "Telugu Sangam MN",
                 "as": "Bangla Sangam MN", "bn": "Bangla Sangam MN", "or": "Oriya Sangam MN",
                 "kn": "Kannada Sangam MN", "ml": "Malayalam Sangam MN", "pa": "Gurmukhi Sangam MN",
                 "si": "Sinhala Sangam MN", "km": "Khmer Sangam MN", "lo": "Lao Sangam MN",
                 "my": "Myanmar Sangam MN"}
        font = None if language == "pa_Arab" else fonts.get(language.split("_")[0])
        if font:
            command.append(f"--font={font}")
    env = dict(os.environ, LANGUAGE=language, QT_QPA_PLATFORM="offscreen", QT_QUICK_BACKEND="software")
    process = subprocess.run(command, env=env, text=True, capture_output=True, timeout=20)
    log = process.stdout + process.stderr
    diagnostics = [line.split("LAYOUT_AUDIT ", 1)[1] for line in log.splitlines() if "LAYOUT_AUDIT " in line]
    result = json.loads(diagnostics[-1]) if diagnostics else {"issues": ["No layout audit"]}
    if page == 9 and args.clock_sync in ["offline", "confirmed"]:
        clock = result.get("timezoneClock", {})
        confirmed = args.clock_sync == "confirmed"
        if clock.get("synchronized") != confirmed:
            result["issues"].append("Clock synchronization gate disagrees with explicit preview mode")
        if not confirmed and (clock.get("time") or clock.get("date")):
            result["issues"].append("Offline preview exposed the time/date readout")
    if state in hostname_examples:
        expected = hostname_examples[state][1]
        actual = len(result.get("hostnameErrors", "").splitlines())
        if actual != expected:
            result["issues"].append(f"Expected {expected} hostname errors, found {actual}")
    result.update(name=name, language=language, screenWidth=width, screenHeight=height,
                  screenshot=snapshot.name, exitCode=process.returncode)
    warnings = [line for line in log.splitlines()
                if "LAYOUT_AUDIT " not in line and "Cannot open: file:///usr/share/pixmaps" not in line
                and "font family" not in line]
    # Keep system font-fallback/KDE diagnostics in the report, separately from
    # actionable QML errors. Never discard missing modules, binding errors or
    # TypeErrors from the installed map or application.
    diagnostics = [line for line in warnings
                   if line.startswith("qt.text.font.db: OpenType support missing for ")
                   or line == "kf.kirigami: OverlayZStacking must be attached to a Popup"]
    if diagnostics:
        result["runtimeDiagnostics"] = diagnostics
        warnings = [line for line in warnings if line not in diagnostics]
    if warnings:
        result["warnings"] = warnings
    if not snapshot.is_file():
        result["issues"].append("Missing screenshot")
    return result

with ThreadPoolExecutor(max_workers=args.workers) as pool:
    results = list(pool.map(render, sorted(jobs)))
report = {"renders": len(results), "results": results}
(args.output / "layout-audit.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
failures = [result for result in results if result["issues"] or result["exitCode"] or result.get("warnings")]
print(json.dumps({"renders": len(results), "failures": failures}, ensure_ascii=False, indent=2))
raise SystemExit(bool(failures))
