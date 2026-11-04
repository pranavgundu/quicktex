export const starter = String.raw`\documentclass{article}
\usepackage{amsmath}

\title{A First Document}
\author{Your Name}

\begin{document}
\maketitle

\begin{abstract}
Your pages are typeset as you type. Edit this text and watch them change.
\end{abstract}

\section{Getting started}
Write ordinary paragraphs and they are justified and broken into lines for you. Leave a blank line to start a new paragraph. Inline math like $e^{i\pi} + 1 = 0$ sits inside the text, and \textbf{bold}, \emph{italic} and \texttt{typewriter} styles work too.

Displayed equations are centered:
\[ \int_{-\infty}^{\infty} e^{-x^2}\,dx = \sqrt{\pi} \]

\subsection{Numbered equations}
The \texttt{equation} environment numbers itself:
\begin{equation}
\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}
\end{equation}

\section{Lists}
\begin{itemize}
\item Press \texttt{Download PDF} to save these pages.
\item Your work is saved in this browser automatically.
\item Lists can nest:
  \begin{enumerate}
  \item first
  \item second
  \end{enumerate}
\end{itemize}

\end{document}
`
