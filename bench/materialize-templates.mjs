// Materialize upstream generator outputs and snippet examples without compiling,
// publishing, or invoking upstream release scripts. All writes stay in .work.
import { readFile, writeFile, mkdir, cp, rm } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { join, dirname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';

const base = dirname(fileURLToPath(import.meta.url));
const work = join(base, '.work/latextemplates');
const inventory = JSON.parse(await readFile(join(base, 'templates-manifest.json'), 'utf8'));
const repository = name => inventory.find(r => r.repository === name);
const pinned = (name, sha = repository(name).sha) => join(work, 'pinned', name, sha);
const generated = [];
const failures = [];
async function setupGenerator(directory) {
  if (!existsSync(join(directory, 'node_modules/yeoman-test'))) {
    execFileSync('npm', ['ci', '--ignore-scripts', '--no-audit', '--no-fund'], { cwd: directory, stdio: 'inherit' });
  }
  const target = join(directory, 'node_modules/yeoman-option-or-prompt/index.js');
  if (!(await readFile(target, 'utf8')).includes('isWhenConditionFulfilled')) {
    execFileSync('patch', ['--forward', '-l', '-p1', '-i', 'patches/yeoman-option-or-prompt+2.0.1.patch'], { cwd: directory, stdio: 'inherit' });
  }
}
const generator = pinned('generator-latex-template');
await setupGenerator(generator);
const requireGenerator = createRequire(join(generator, 'package.json'));
const helpers = (await import(pathToFileURL(requireGenerator.resolve('yeoman-test')))).default;
const matrix = await import(pathToFileURL(join(generator, '__tests__/matrix.js')));
const combinations = matrix.pairwiseMatrix();
// The upstream pairwise matrix omits the scientific-thesis option.
for (const language of ['en', 'de']) combinations.push({ ...combinations[0], documentclass: 'scientific-thesis', language, latexcompiler: 'both', bibtextool: 'biblatex', font: 'times', example: 'true', howtotext: 'true' });
for (let index = 0; index < combinations.length; index++) {
  const combo = combinations[index];
  const id = `matrix-${String(index).padStart(2, '0')}`;
  const directory = `generated/generator/${id}`;
  try {
    const result = await helpers.run(join(generator, 'generators/app')).withOptions(matrix.toOptions(combo));
    await rm(join(work, directory), { recursive: true, force: true });
    await cp(result.cwd, join(work, directory), { recursive: true });
    result.cleanup();
    generated.push({ repository: `generator-latex-template/${id}`, upstream: 'generator-latex-template', sha: repository('generator-latex-template').sha, directory, roots: [combo.documentclass === 'scientific-thesis' ? `main-${combo.language === 'de' ? 'german' : 'english'}.tex` : matrix.mainFile(combo)], options: matrix.toOptions(combo) });
    console.log('Generated', id, matrix.label(combo));
  } catch (error) { failures.push({ repository: 'generator-latex-template', id, error: String(error) }); }
}

// Resolve the actual gitlink revision, rather than substituting today's generator.
const snippetRepo = repository('latex-snippets');
const response = await fetch(`https://api.github.com/repos/latextemplates/latex-snippets/git/trees/${snippetRepo.sha}`);
if (!response.ok) throw new Error(`Cannot resolve snippet submodule: ${response.status}`);
const submoduleSha = (await response.json()).tree.find(item => item.path === 'generator-latex-template' && item.mode === '160000')?.sha;
if (!submoduleSha) throw new Error('Missing pinned snippet generator gitlink');
const snippetGenerator = pinned('generator-latex-template', submoduleSha);
if (!existsSync(snippetGenerator)) {
  await mkdir(snippetGenerator, { recursive: true });
  const archive = join(work, 'snippet-generator.tar.gz');
  const archiveResponse = await fetch(`https://api.github.com/repos/latextemplates/generator-latex-template/tarball/${submoduleSha}`);
  if (!archiveResponse.ok) throw new Error(`Cannot download snippet generator: ${archiveResponse.status}`);
  await writeFile(archive, Buffer.from(await archiveResponse.arrayBuffer()));
  execFileSync('tar', ['-xzf', archive, '-C', snippetGenerator, '--strip-components=1']);
  await rm(archive);
}
await setupGenerator(snippetGenerator);
const snippetRequire = createRequire(join(snippetGenerator, 'package.json'));
// Expose the upstream builder's own render functions. Its normal main (Docker,
// SVGs and site writes) is not run. The source files remain unchanged.
let runtime = await readFile(join(pinned('latex-snippets'), 'scripts/build-snippets.mjs'), 'utf8');
runtime = runtime.replace('import ejs from "ejs";', `import ejs from ${JSON.stringify(pathToFileURL(snippetRequire.resolve('ejs')).href)};`)
  .replace('const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");', `const ROOT = ${JSON.stringify(pinned('latex-snippets'))};`)
  .replace('const BUILD = join(ROOT, "build");', `const BUILD = ${JSON.stringify(join(work, 'generated/snippet-bases'))};`);
const mainCall = runtime.indexOf('\nmain().catch(');
if (mainCall < 0) throw new Error('Upstream snippet entry point changed');
runtime = runtime.slice(0, mainCall) + '\nexport { getBase, EXAMPLE_PROPS, renderTemplate, resolveSrc, contextDoc };\n';
const runtimePath = join(work, 'snippet-runtime.mjs');
await writeFile(runtimePath, runtime);
process.env.GENERATOR_DIR = snippetGenerator;
const api = await import(pathToFileURL(runtimePath));
const { snippets } = await import(pathToFileURL(join(pinned('latex-snippets'), 'snippets.config.mjs')));
for (const locale of ['en', 'de']) {
  for (const [slug, meta] of Object.entries(snippets)) {
    const id = `${slug}-${locale}`;
    try {
      const src = api.resolveSrc(meta.stem ?? slug, 'example', locale);
      if (!src) {
        // Preamble-only catalog entries are configuration examples, with no
        // output fragment. Keep them explicitly in the materialization audit.
        failures.push({ repository: 'latex-snippets', id, status: 'preamble-only' });
        continue;
      }
      const lang = src.localized ? locale : 'en';
      const cfg = meta.config ?? {};
      const props = { ...api.EXAMPLE_PROPS, ...cfg, lang, language: lang, bexample: '', eexample: '', heading2: '\\section' };
      props.bquote = props.enquotes === 'textcmds' ? '\\qq{' : '\\enquote{';
      props.equote = '}';
      const cleveref = props.crossref === 'cleveref';
      props.crossref = cleveref ? 'cleveref' : 'zref-clever';
      props.cref = cleveref ? '\\cref' : '\\zcref'; props.Cref = cleveref ? '\\Cref' : '\\Zcref';
      props.vref = cleveref ? '\\vref' : '\\zvref'; props.Vref = cleveref ? '\\Vref' : '\\zvref[S]';
      const { genDir, preamble } = await api.getBase(cfg, lang);
      const example = await api.renderTemplate(src.file, props);
      const directory = `generated/snippets/${id}`;
      await rm(join(work, directory), { recursive: true, force: true });
      await cp(genDir, join(work, directory), { recursive: true });
      const innerClass = ['scientific-thesis', 'ustutt'].includes(cfg.documentclass) ? 'scrbook' : null;
      await writeFile(join(work, directory, 'snippet.tex'), api.contextDoc(preamble, example, innerClass));
      generated.push({ repository: `latex-snippets/${id}`, upstream: 'latex-snippets', sha: snippetRepo.sha, generatorSha: submoduleSha, directory, roots: ['snippet.tex'], template: src.file, config: cfg });
      console.log('Generated snippet', id);
    } catch (error) { failures.push({ repository: 'latex-snippets', id, error: String(error) }); console.error('Failed snippet', id, String(error)); }
  }
}
// The distributable Handin archive includes the stylesheet and logo alongside
// its examples; its raw src/ directory is not the installed package layout.
const handinDirectory = 'generated/handin-distribution';
await rm(join(work, handinDirectory), { recursive: true, force: true });
await mkdir(join(work, handinDirectory), { recursive: true });
execFileSync('unzip', ['-q', join(pinned('Handin-LaTeX-template'), 'handin.zip'), '-d', join(work, handinDirectory)]);
generated.push({ repository: 'Handin-LaTeX-template/distribution', upstream: 'Handin-LaTeX-template', sha: repository('Handin-LaTeX-template').sha, directory: handinDirectory + '/handin', roots: ['example.tex', 'handin-doc.tex'], kind: 'packaged-distribution' });

// This repository ships a class rather than a document. Exercise its public
// title/abstract/section interface with a smoke document, labelled as such.
const stysDirectory = 'generated/stys-class-smoke';
await rm(join(work, stysDirectory), { recursive: true, force: true });
await cp(pinned('stys-for-overleaf'), join(work, stysDirectory), { recursive: true });
await writeFile(join(work, stysDirectory, 'smoke.tex'), String.raw`\documentclass{llncs}
\title{Class compatibility check}\author{A. Author}\institute{Example Institute}
\begin{document}\maketitle
\begin{abstract}A minimal document exercises the class distributed by this repository.\end{abstract}
\section{Introduction}A paragraph with inline math $a^2+b^2=c^2$.
\end{document}
`);
generated.push({ repository: 'stys-for-overleaf/class-smoke', upstream: 'stys-for-overleaf', sha: repository('stys-for-overleaf').sha, directory: stysDirectory, roots: ['smoke.tex'], kind: 'class-smoke' });
for (const project of generated) {
  const listing = execFileSync('python3', ['-c', 'import pathlib,hashlib,sys; p=pathlib.Path(sys.argv[1]); h=hashlib.sha256(); [(h.update(str(f.relative_to(p)).encode()+bytes([0])),h.update(f.read_bytes())) for f in sorted(p.rglob("*")) if f.is_file() and "node_modules" not in f.parts and ".git" not in f.parts]; print(h.hexdigest())', join(work, project.directory)], { encoding: 'utf8' });
  project.contentHash = listing.trim();
}
await writeFile(join(base, 'templates-generated-manifest.json'), JSON.stringify(generated, null, 2) + '\n');
await writeFile(join(work, 'materialization.json'), JSON.stringify({ generated: generated.length, documents: generated.reduce((n, project) => n + project.roots.length, 0), failures }, null, 2) + '\n');
console.log('Materialized', generated.reduce((n, project) => n + project.roots.length, 0), 'documents;', failures.length, 'entries need inspection');

for (const directory of new Set([generator, snippetGenerator])) await rm(join(directory, 'node_modules'), { recursive: true, force: true });
