const sentences = [
  'The measured response follows the model closely across the whole range of inputs.',
  'A small change in the boundary condition moves the first root only slightly.',
  'Every sample was taken at the same temperature and stored in the same way.',
  'The second method needs fewer steps but keeps more values in memory at once.',
  'Errors grow slowly at first and then settle near a constant value.',
  'We repeat the procedure until two runs in a row agree to four digits.',
  'This bound is tight for small inputs and loose for large ones.',
  'The table below lists the values used in every later section of this report.',
  'A longer interval smooths the noise but hides the short peaks in the signal.',
  'Both estimates agree once the sample holds more than a few hundred points.',
]

const equations = [
  String.raw`\int_0^\infty e^{-x^2}\,dx = \frac{\sqrt{\pi}}{2}`,
  String.raw`\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}`,
  String.raw`x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}`,
  String.raw`\frac{\partial u}{\partial t} = \alpha \left( \frac{\partial^2 u}{\partial x^2} + \frac{\partial^2 u}{\partial y^2} \right)`,
  String.raw`\begin{pmatrix} a & b \\ c & d \end{pmatrix} \begin{pmatrix} x \\ y \end{pmatrix} = \begin{pmatrix} ax + by \\ cx + dy \end{pmatrix}`,
  String.raw`\prod_{p \le n} \left( 1 - \frac{1}{p} \right)^{-1} \ge \log n`,
  String.raw`\lim_{h \to 0} \frac{f(x + h) - f(x)}{h} = f'(x)`,
  String.raw`\oint_C \mathbf{F} \cdot d\mathbf{r} = \iint_S (\nabla \times \mathbf{F}) \cdot d\mathbf{S}`,
]

const words = (count: number, seed: number) => {
  const out: string[] = []
  for (let i = 0; i < count; i++) out.push(sentences[(seed + i * 3) % sentences.length])
  return out.join(' ')
}

const paragraphs = (count: number, seed = 0) =>
  Array.from({ length: count }, (_, i) => `${words(5, seed + i)}\n`).join('\n')

const article = (preamble: string, body: string) =>
  `\\documentclass[11pt]{article}\n\\usepackage{amsmath}\n\\usepackage[margin=1in]{geometry}\n${preamble}\n\\begin{document}\n${body}\n\\end{document}\n`

const table = String.raw`\begin{tabular}{lrr}
\toprule
Sample & Mean & Spread \\
\midrule
First & 1.25 & 0.04 \\
Second & 1.31 & 0.07 \\
Third & 1.18 & 0.02 \\
\bottomrule
\end{tabular}`

function note() {
  return article('', `Dear reader,\n\n${paragraphs(3)}\nKind regards,\n\nThe authors`)
}

function paper() {
  const section = (i: number) => `\\section{Part ${i + 1}}\\label{part${i}}
${paragraphs(4, i)}
\\begin{equation}\\label{eq${i}}
${equations[i % equations.length]}
\\end{equation}
As shown in Section~\\ref{part${(i + 1) % 8}} and Equation~\\eqref{eq${i}}, ${words(2, i)}\\footnote{${words(1, i)}}
\\begin{itemize}
\\item ${words(1, i)}
\\item ${words(1, i + 1)}
\\end{itemize}
\\begin{center}
${table}
\\end{center}
${paragraphs(2, i + 3)}`
  const body = `\\title{Measured Response of Coupled Systems}
\\author{A. Writer}
\\date{March 2026}
\\maketitle
\\begin{abstract}
${words(4, 1)}
\\end{abstract}
${Array.from({ length: 8 }, (_, i) => section(i)).join('\n')}`
  return article('\\usepackage{booktabs}', body)
}

function formulas() {
  const body = Array.from({ length: 400 }, (_, i) => `\\begin{equation}\n${equations[i % equations.length]}\n\\end{equation}`).join('\n')
  return article('', body)
}

function book() {
  const section = (i: number) => `\\section{Chapter ${i + 1}}\\label{s${i}}
${paragraphs(10, i)}
\\subsection{Results of chapter ${i + 1}}
${paragraphs(6, i + 2)}
\\begin{equation}
${equations[i % equations.length]}
\\end{equation}
See Section~\\ref{s${(i + 7) % 50}}. ${paragraphs(6, i + 5)}`
  return article('', `\\tableofcontents\n\\newpage\n${Array.from({ length: 50 }, (_, i) => section(i)).join('\n')}`)
}

function drawing() {
  const shapes = Array.from({ length: 600 }, (_, i) => {
    const x = (i % 30) * 0.5
    const y = Math.floor(i / 30) * 0.5
    if (i % 3 === 0) return `\\draw (${x},${y}) circle (0.2);`
    if (i % 3 === 1) return `\\draw[thick] (${x},${y}) -- (${x + 0.4},${y + 0.4});`
    return `\\fill[blue] (${x},${y}) rectangle (${x + 0.3},${y + 0.3});`
  })
  return article('\\usepackage{tikz}', `\\begin{center}\n\\begin{tikzpicture}[scale=0.9]\n${shapes.join('\n')}\n\\end{tikzpicture}\n\\end{center}`)
}

function plots() {
  const plot = (i: number) => String.raw`\begin{tikzpicture}
\begin{axis}[width=10cm, height=6cm, xlabel={$x$}, ylabel={$y$}, grid=major]
\addplot[blue, domain=0:6, samples=60] {sin(deg(x)) + ${i} * 0.1};
\addplot[red, domain=0:6, samples=60] {cos(deg(x))};
\end{axis}
\end{tikzpicture}`
  const body = Array.from({ length: 8 }, (_, i) => `\\begin{center}\n${plot(i)}\n\\end{center}`).join('\n')
  return article('\\usepackage{pgfplots}\n\\pgfplotsset{compat=1.18}', body)
}

function tree() {
  return String.raw`\documentclass[border=10pt]{standalone}
\usepackage{tikz}
\usepackage{xcolor}
\definecolor{treegreen}{RGB}{20,130,70}
\newcommand{\fractaltree}[1]{%
    \ifnum#1>0
        \pgfmathtruncatemacro{\next}{#1-1}
        \pgfmathsetmacro{\len}{0.72^(12-#1)}
        \draw[treegreen, line width=0.25pt] (0,0) -- (0,\len);
        \begin{scope}[shift={(0,\len)}]
            \begin{scope}[rotate=25]
                \fractaltree{\next}
            \end{scope}
            \begin{scope}[rotate=-25]
                \fractaltree{\next}
            \end{scope}
        \end{scope}
    \fi
}
\begin{document}
\begin{tikzpicture}
    \begin{scope}[scale=3]
        \fractaltree{12}
    \end{scope}
\end{tikzpicture}
\end{document}
`
}

export const documents: Record<string, { about: string; passes: boolean; tex: () => string }> = {
  note: { about: 'one page of text', passes: false, tex: note },
  paper: { about: 'paper with math, tables, footnotes and references', passes: true, tex: paper },
  formulas: { about: '400 numbered equations', passes: false, tex: formulas },
  book: { about: 'long report with contents and references', passes: true, tex: book },
  drawing: { about: 'TikZ picture with 600 shapes', passes: false, tex: drawing },
  plots: { about: 'eight pgfplots charts', passes: false, tex: plots },
  tree: { about: 'recursive TikZ fractal with 4,095 branches', passes: false, tex: tree },
}
