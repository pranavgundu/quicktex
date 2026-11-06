#!/usr/bin/env python3
"""Render the upstream latextemplates corpus without modifying its sources.

The report is diagnostic: an emitted PDF is not a compatibility pass. Resume
reuses only completed jobs for the same source revision and compiler binary.
"""
import argparse
import concurrent.futures
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import tarfile
import time
import urllib.request

BASE = Path(__file__).resolve().parent
WORK = BASE / '.work' / 'latextemplates'
ENV = dict(os.environ, PATH=str(WORK / 'bin') + ':/Library/TeX/texbin:' + os.environ.get('PATH', ''))
MANIFEST = BASE / 'templates-manifest.json'
TOOLS = ['quicktex', 'pdflatex', 'xelatex', 'lualatex', 'tectonic']


def fetch(url):
    with urllib.request.urlopen(urllib.request.Request(url, headers={'User-Agent': 'quicktex-corpus'}), timeout=120) as response:
        return response.read()


def prepare(refresh=False):
    WORK.mkdir(parents=True, exist_ok=True)
    pinned = {} if refresh or not MANIFEST.exists() else {r['repository']: r['sha'] for r in json.loads(MANIFEST.read_text())}
    repos = [{'name': name} for name in pinned] if pinned else json.loads(fetch('https://api.github.com/orgs/latextemplates/repos?per_page=100'))
    manifest = []
    for repo in repos:
        name = repo['name']
        sha = pinned.get(name) or json.loads(fetch(f"https://api.github.com/repos/latextemplates/{name}/commits/{repo['default_branch']}"))['sha']
        dest = WORK / 'pinned' / name / sha
        if not dest.exists():
            archive = WORK / f'{name}.tar.gz'
            archive.write_bytes(fetch(f'https://api.github.com/repos/latextemplates/{name}/tarball/{sha}'))
            dest.mkdir(parents=True)
            with tarfile.open(archive) as tar:
                # GitHub archives have a single enclosing directory.
                for member in tar.getmembers():
                    member.name = '/'.join(member.name.split('/')[1:])
                    if member.name:
                        tar.extract(member, dest, filter='data')
            archive.unlink()
        roots = []
        source_templates = []
        working_directories = {}
        for path in sorted(dest.rglob('*.tex')):
            if 'node_modules' in path.parts:
                continue
            raw_source = path.read_text(errors='replace')
            source = re.sub(r'(?m)(?<!\\)%.*$', '', raw_source)
            if re.search(r'\\(?:documentclass|documentstyle)\b', source):
                if '<%' in raw_source:
                    source_templates.append(str(path.relative_to(dest)))
                else:
                    entry = str(path.relative_to(dest))
                    roots.append(entry)
                    if entry.startswith('comparison/') and re.search(r'\\(?:input|include)\s*\{comparison/', source):
                        working_directories[entry] = '.'
        manifest.append({'repository': name, 'sha': sha, 'roots': roots, 'sourceTemplates': source_templates, 'workingDirectories': working_directories})
        print(name, sha[:12], len(roots), 'document roots', flush=True)
    MANIFEST.write_text(json.dumps(manifest, indent=2) + '\n')


def needs_shell_escape(project, source):
    configuration = '\n'.join(path.read_text(errors='replace') for path in project.rglob('.latexmkrc'))
    text = source.read_text(errors='replace')
    return '--shell-escape' in configuration or '--shell-escape' in text or bool(re.search(r'\\usepackage(?:\[[^]]*\])?\{[^}]*\b(?:minted|markdown|plantuml|ltmermaid)\b', text))


def execute(command, cwd, timeout):
    start = time.monotonic()
    process = subprocess.Popen(command, cwd=cwd, env=ENV, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, start_new_session=True)
    try:
        output, _ = process.communicate(timeout=timeout)
        return {'exitCode': process.returncode, 'seconds': time.monotonic() - start}, output.decode(errors='replace')
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        output, _ = process.communicate()
        return {'timeout': timeout, 'seconds': time.monotonic() - start}, output.decode(errors='replace')



def run_job(repo, entry, tool, timeout, resume, only_pending=False, retry_log=None):
    source = WORK / repo['directory'] if 'directory' in repo else WORK / 'pinned' / repo['repository'] / repo['sha']
    key = hashlib.sha256(entry.encode()).hexdigest()[:12]
    directory = WORK / 'runs' / repo['repository'] / key / tool
    resultfile = directory / 'result.json'
    binary = BASE.parent / 'wasm-core/js/quicktex.wasm' if tool == 'quicktex' else Path(shutil.which(tool, path=ENV['PATH']) or '/nonexistent')
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest() if binary.exists() else 'missing'
    dependencies = [BASE / 'templates.py', BASE / 'template-compile.ts'] if tool == 'quicktex' else [BASE / 'templates.py', Path(shutil.which('biber', path=ENV['PATH']) or '/nonexistent'), Path(shutil.which('latexminted', path=ENV['PATH']) or '/nonexistent')]
    dependency_hash = hashlib.sha256(b''.join(p.read_bytes() for p in dependencies if p.exists())).hexdigest()
    identity = {'recipe': 2, 'materialization': repo.get('contentHash'), 'runner': dependency_hash, 'repository': repo['repository'], 'revision': repo['sha'], 'entry': entry, 'compiler': tool, 'binary': fingerprint}
    if tool == 'tectonic':
        identity['recipe'] = 3
    working_directory = repo.get('workingDirectories', {}).get(entry)
    if working_directory is not None:
        identity['workingDirectory'] = working_directory
    if tool == 'tectonic' and working_directory is not None:
        identity['searchDirectory'] = working_directory
    if tool == 'quicktex':
        identity['fontBinary'] = hashlib.sha256((BASE.parent / 'wasm-core/js/quicktex-fonts.wasm').read_bytes()).hexdigest()
    if resume and resultfile.exists():
        old = json.loads(resultfile.read_text())
        checked_identity = identity if not only_pending else {k: v for k, v in identity.items() if k != 'runner'}
        compiler_log = directory / 'compiler.log'
        retry = retry_log and old.get('status') == 'failed' and compiler_log.exists() and re.search(retry_log, compiler_log.read_text(errors='replace'))
        if all(old.get(k) == v for k, v in checked_identity.items()) and not retry:
            return old
    if directory.exists():
        shutil.rmtree(directory)
    directory.mkdir(parents=True)
    if fingerprint == 'missing':
        result = dict(identity, status='unavailable')
    else:
        project = directory / 'project'
        shutil.copytree(source, project, ignore=shutil.ignore_patterns('node_modules', '.git', 'target'))
        cwd = project / (working_directory if working_directory is not None else str(Path(entry).parent))
        name = os.path.relpath(project / entry, cwd)
        pdf = cwd / (Path(name).stem + '.pdf')
        # Remove shipped PDFs so a failed compiler cannot inherit a successful output.
        pdf.unlink(missing_ok=True)
        if tool == 'quicktex':
            command = ['bun', str(BASE / 'template-compile.ts'), str(cwd / name), str(project), str(pdf)]
        elif tool == 'tectonic':
            command = ['tectonic', '--keep-logs', '--keep-intermediates', '--outdir', str(cwd), '-Z', f'search-path={cwd}']
            if needs_shell_escape(project, cwd / name):
                command += ['-Z', f'shell-escape-cwd={cwd}']
            command.append(name)
        else:
            # .latexmkrc may assign lualatex to $pdflatex. Force the engine
            # being compared while retaining the project's other build rules.
            shell_flag = '-shell-escape' if needs_shell_escape(project, cwd / name) else '-no-shell-escape'
            command = ['latexmk', '-jobname=corpus-output', {'pdflatex': '-pdf', 'xelatex': '-xelatex', 'lualatex': '-lualatex'}[tool], f'-{tool}={tool} %O %S', '-interaction=nonstopmode', '-halt-on-error', '-file-line-error', shell_flag, name]
        if tool in ['pdflatex', 'xelatex', 'lualatex']:
            pdf = cwd / 'corpus-output.pdf'
            pdf.unlink(missing_ok=True)
        metrics, log = execute(command, cwd, timeout)
        (directory / 'compiler.log').write_text(log)
        result = dict(identity, command=command, **metrics)
        result['status'] = 'timeout' if 'timeout' in metrics else 'failed'
        if tool == 'quicktex':
            for line in reversed(log.splitlines()):
                try:
                    result.update(json.loads(line))
                    break
                except (ValueError, TypeError):
                    pass
        if metrics.get('exitCode') == 0 and pdf.exists():
            info, info_log = execute(['pdfinfo', str(pdf)], cwd, 15)
            match = re.search(r'Pages:\s+(\d+)', info_log)
            if info.get('exitCode') == 0 and match:
                result.update(status='rendered', pages=int(match[1]), pdf=str(pdf.relative_to(WORK)), pdfBytes=pdf.stat().st_size)
                _, text = execute(['pdftotext', '-layout', str(pdf), '-'], cwd, 15)
                (directory / 'text.txt').write_text(text)
                result['textCharacters'] = len(text.strip())
                if tool == 'quicktex' and result.get('warnings'):
                    result['status'] = 'warnings'
    resultfile.write_text(json.dumps(result, indent=2) + '\n')
    print(f"{result['status']:10} {tool:9} {repo['repository']}/{entry}", flush=True)
    return result


def report(manifest):
    binaries = {}
    for tool in TOOLS:
        binary = BASE.parent / 'wasm-core/js/quicktex.wasm' if tool == 'quicktex' else Path(shutil.which(tool, path=ENV['PATH']) or '/nonexistent')
        binaries[tool] = hashlib.sha256(binary.read_bytes()).hexdigest() if binary.exists() else 'missing'
    font_binary = hashlib.sha256((BASE.parent / 'wasm-core/js/quicktex-fonts.wasm').read_bytes()).hexdigest()
    rows = []
    for repo in manifest:
        for entry in repo['roots']:
            key = hashlib.sha256(entry.encode()).hexdigest()[:12]
            for tool in TOOLS:
                path = WORK / 'runs' / repo['repository'] / key / tool / 'result.json'
                result = json.loads(path.read_text()) if path.exists() else {}
                if (result.get('revision') != repo['sha'] or result.get('binary') != binaries[tool]
                        or result.get('materialization') != repo.get('contentHash')
                        or (entry in repo.get('workingDirectories', {}) and result.get('workingDirectory') != repo['workingDirectories'][entry])
                        or (tool == 'tectonic' and entry in repo.get('workingDirectories', {}) and result.get('searchDirectory') != repo['workingDirectories'][entry])
                        or (tool == 'quicktex' and result.get('fontBinary') != font_binary)
                        or (tool != 'quicktex' and result.get('recipe') != (3 if tool == 'tectonic' else 2))):
                    result = {}
                rows.append(dict(result, repository=repo['repository'], entry=entry, compiler=tool, status=result.get('status', 'pending')))
    (WORK / 'results.json').write_text(json.dumps(rows, indent=2) + '\n')
    lines = ['# Upstream template rendering', '',
             f"Pinned {len({r.get('upstream', r['repository']) for r in manifest})} repositories and {sum(len(r['roots']) for r in manifest)} document roots in `templates-manifest.json` and `templates-generated-manifest.json` (when present). Generated projects use the upstream generator and snippet catalog.", '',
             'These are diagnostic renders. “rendered” means the process finished and produced a valid PDF; it does not prove content or visual equivalence. Warnings count separately. Runs execute concurrently, so elapsed times are not a speed ranking. Engine-specific source requirements and locally available dependencies also affect the render counts.', '',
             '| Compiler | Rendered | With warnings | Failed | Timeout | Pending |',
             '| --- | ---: | ---: | ---: | ---: | ---: |']
    for tool in TOOLS:
        values = [sum(r['compiler'] == tool and r['status'] == status for r in rows) for status in ['rendered', 'warnings', 'failed', 'timeout', 'pending']]
        lines.append('| ' + tool + ' | ' + ' | '.join(map(str, values)) + ' |')
    comparisons = []
    for repo in manifest:
        for entry in repo['roots']:
            candidates = [r for r in rows if r['repository'] == repo['repository'] and r['entry'] == entry]
            quick = next((r for r in candidates if r['compiler'] == 'quicktex' and r.get('pdf')), None)
            if not quick:
                continue
            # Output directories can be nested under the project; use the job key.
            key = hashlib.sha256(entry.encode()).hexdigest()[:12]
            folder = WORK / 'runs' / repo['repository'] / key
            quick_words = Counter(re.findall(r"\w+", (folder / 'quicktex/text.txt').read_text().lower()))
            for reference in candidates:
                if reference['compiler'] == 'quicktex' or reference['status'] != 'rendered':
                    continue
                reference_words = Counter(re.findall(r"\w+", (folder / reference['compiler'] / 'text.txt').read_text().lower()))
                common = sum((quick_words & reference_words).values())
                comparisons.append({'repository': repo['repository'], 'entry': entry, 'reference': reference['compiler'],
                    'quicktexPages': quick['pages'], 'referencePages': reference['pages'],
                    'samePages': quick['pages'] == reference['pages'], 'sameWords': quick_words == reference_words,
                    'referenceWordRecall': common / max(1, sum(reference_words.values())),
                    'quicktexWarnings': len(quick.get('warnings', [])), 'visualReview': 'required'})
    (WORK / 'comparisons.json').write_text(json.dumps(comparisons, indent=2) + '\n')
    lines += ['', '## Fidelity checks', '',
              f"Compared {len(comparisons)} QuickTeX/reference PDF pairs: {sum(not c['samePages'] for c in comparisons)} page-count mismatches and {sum(not c['sameWords'] for c in comparisons)} extracted-word mismatches.", '',
              'Word comparison uses lowercased token counts; it cannot validate ordering, equations, images, typography or layout. All pairs still require visual review. Per-pair metrics are in `.work/latextemplates/comparisons.json`.', '',
              'Visual spot check: `scientific-thesis-template/chapters-en/10 - introduction.tex` has different fonts, margins, list spacing and unresolved reference text in QuickTeX; its 4 pages differ from pdfLaTeX’s 5. It is not compatible.', '']
    pagination = []
    for row in rows:
        if row['status'] != 'timeout':
            continue
        key = hashlib.sha256(row['entry'].encode()).hexdigest()[:12]
        log = WORK / 'runs' / row['repository'] / key / row['compiler'] / 'compiler.log'
        markers = [int(n) for n in re.findall(r'\[(\d+)\]', log.read_text(errors='replace'))] if log.exists() else []
        if len(markers) >= 1000:
            pagination.append({'repository': row['repository'], 'entry': row['entry'], 'compiler': row['compiler'], 'pageMarkers': len(markers), 'largestPageMarker': max(markers)})
    (WORK / 'timeout-audit.json').write_text(json.dumps(pagination, indent=2) + '\n')
    if pagination:
        lines += ['', '## Timeout diagnostics', '',
                  'These jobs emitted thousands of page markers before the process-group timeout. Their pagination behavior needs investigation; a longer timeout alone is not a useful remedy.', '',
                  '| Project | Compiler | Largest page marker |', '| --- | --- | ---: |']
        for row in pagination:
            lines.append(f"| {row['repository']}/{row['entry']} | {row['compiler']} | {row['largestPageMarker']} |")
    lines += ['', '## Repository coverage', '', '| Repository | Roots |', '| --- | ---: |']
    for repo in manifest:
        upstream = repo.get('upstream', repo['repository'])
        lines.append(f"| [{repo['repository']}](https://github.com/latextemplates/{upstream}/tree/{repo['sha']}) | {len(repo['roots'])} |")
    lines += ['', '## QuickTeX errors', '']
    errors = {}
    for row in rows:
        if row['compiler'] == 'quicktex' and row.get('error'):
            errors[row['error']] = errors.get(row['error'], 0) + 1
    for error, count in sorted(errors.items(), key=lambda item: -item[1]):
        lines.append(f'- {count} documents: {error}')
    lines += ['', 'Full per-document results, PDFs, extracted text and logs are in `bench/.work/latextemplates/`. Raw EJS source templates are inventoried separately; generator outputs, snippet context documents and the class smoke test are listed as generated projects. See `materialization.json` for preamble-only catalog entries that have no example to typeset.', '']
    (BASE / 'templates-results.md').write_text('\n'.join(lines))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--prepare', action='store_true')
    parser.add_argument('--refresh', action='store_true', help='Update pinned upstream revisions with --prepare')
    parser.add_argument('--report-only', action='store_true')
    parser.add_argument('--tools', nargs='+', choices=TOOLS, default=TOOLS)
    parser.add_argument('--repositories', nargs='+')
    parser.add_argument('--jobs', type=int, default=2)
    parser.add_argument('--timeout', type=int, default=90)
    parser.add_argument('--resume', action='store_true')
    parser.add_argument('--retry-log', help='With resume/only-pending, retry failed jobs whose compiler log matches this regular expression')
    parser.add_argument('--only-pending', action='store_true', help='Keep completed jobs with matching source and binary even if runner code changed; use after auditing runner changes')
    args = parser.parse_args()
    if args.prepare:
        prepare(args.refresh)
    manifest = json.loads(MANIFEST.read_text())
    generated = BASE / 'templates-generated-manifest.json'
    if generated.exists():
        manifest += json.loads(generated.read_text())
    if args.report_only:
        report(manifest)
        return
    jobs = [(repo, entry, tool) for repo in manifest if not args.repositories or repo['repository'] in args.repositories or repo.get('upstream') in args.repositories for entry in repo['roots'] for tool in args.tools]
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as pool:
        results = list(pool.map(lambda job: run_job(*job, args.timeout, args.resume or args.only_pending, args.only_pending, args.retry_log), jobs))
    (WORK / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
    counts = {tool: {status: sum(r['compiler'] == tool and r['status'] == status for r in results) for status in ['rendered', 'warnings', 'failed', 'timeout', 'unavailable']} for tool in args.tools}
    print(json.dumps(counts, indent=2))
    report(manifest)


if __name__ == '__main__':
    main()
