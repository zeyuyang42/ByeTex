### Basic Natbib Setup and Citation

Source: https://www.overleaf.com/learn/latex/Natbib_citation_styles

This example demonstrates the basic setup of the natbib package, including setting the bibliography style and citation style. It shows how to use \cite and \citep commands with optional arguments for pre- and post-notes.

```latex
\documentclass{article}
\usepackage[utf8]{inputenc}
\usepackage[english]{babel}

%Import the natbib package and sets a bibliography  and citation styles
\usepackage{natbib}
\bibliographystyle{abbrvnat}
\setcitestyle{authoryear,open={((},close={))}} %Citation-related commands

\title{Natbib Example}

\author{Overleaf team}

\begin{document}

\maketitle

\section{First Section}
This document is an example with two cited items: \textit{The \LaTeX\ Companion} book \cite[see][chap 2]{latexcompanion} and Einstein's journal paper \citep{einstein}. 


%Imports the bibliography file "sample.bib"
\bibliography{sample}

\end{document}


```

--------------------------------

### Minimal biblatex Setup

Source: https://www.overleaf.com/learn/latex/Basic_bibliography_management

A minimal working example demonstrating the basic setup for the biblatex package. It includes importing the package, adding a bibliography resource, and printing the bibliography.

```latex
\documentclass[letterpaper,10pt]{article}
\usepackage{biblatex} %Imports biblatex package
\addbibresource{sample.bib} %Import the bibliography file

\begin{document}
Let's cite! Einstein's journal paper \cite{einstein} and Dirac's
book \cite{dirac} are physics-related items. 

\printbibliography %Prints bibliography

\end{document}

```

--------------------------------

### Basic natbib setup and citation

Source: https://www.overleaf.com/learn/latex/Bibliography_management_with_natbib

Imports the natbib package and sets the bibliography style to unsrtnat. Includes example citations and bibliography import.

```latex
\usepackage{natbib}
\bibliographystyle{unsrtnat}
\title{Bibliography management: \texttt{natbib} package}
\author{Overleaf}
\date {April 2021}

\begin{document}

\maketitle

This document is an example of \texttt{natbib} package using in bibliography
management. Three items are cited: \textit{The \LaTeX\ Companion} book 
\cite{latexcompanion}, the Einstein journal paper \cite{einstein}, and the 
Donald Knuth's website \cite{knuthwebsite}. The \LaTeX\ related items are 
\cite{latexcompanion,knuthwebsite}. 

\medskip

\bibliography{sample}

\end{document}
```

--------------------------------

### Basic Portuguese Setup with pdfLaTeX

Source: https://www.overleaf.com/learn/latex/Portuguese

This example demonstrates the essential packages and configurations for typesetting Portuguese using pdfLaTeX. Ensure `fontenc` and `babel` are included for correct character rendering and language features. The `hyphenat` package with custom hyphenation rules is also shown.

```latex
\documentclass{article}
% \usepackage[utf8]{inputenc} is no longer required (since 2018)

%Set the font (output) encoding
%--------------------------------------
\usepackage[T1]{fontenc} %Not needed by LuaLaTeX or XeLaTeX
%--------------------------------------

%Portuguese-specific commands
%--------------------------------------
\usepackage[portuguese]{babel}
%--------------------------------------

%Hyphenation rules
%--------------------------------------
\usepackage{hyphenat}
\hyphenation{mate-mática recu-perar}
%--------------------------------------

\begin{document}
\tableofcontents

\vspace{2cm} %Add a 2cm space

\begin{abstract}
Este é um breve resumo do conteúdo do documento escrito em Português.
\end{abstract}

\section{Seção introdutória}
Esta é a primeira seção, podemos acrescentar alguns elementos adicionais 
e tudo será escrito corretamente. Além disso, se uma palavra é um caminho 
muito longo e tem de ser truncado, babel irá tentar truncar corretamente, 
dependendo do idioma.

\section{Segunda seção}
Esta seção é para ver o que acontece com comandos de texto que definem

\[ \lim x =  \theta + 152383.52 \]
\end{document}
```

--------------------------------

### Basic LaTeX Index Creation

Source: https://www.overleaf.com/learn/latex/Indices

This example demonstrates the fundamental setup for creating an index in a LaTeX document. It includes necessary package loading, index generation command, and printing the index.

```latex
\documentclass{article}
\usepackage[T1]{fontenc}
\usepackage{imakeidx}
\makeindex

\begin{document}
\section{Introduction}
In this example, several keywords\index{keywords} will be used 
which are important and deserve to appear in the Index\index{Index}.

Terms like generate\index{generate} and some\index{others} will 
also show up. 

\printindex
\end{document}

```

```latex
\usepackage{imakeidx}

```

--------------------------------

### LaTeX Example Environment with Refstepcounter

Source: https://www.overleaf.com/learn/latex/Counters

Defines a custom 'example' environment that uses \refstepcounter to increment a counter, making it available for \label and \ref. The counter is reset at the start of each new section.

```latex
\documentclass{article}

\newcounter{example}[section]
\newenvironment{example}[1][]{\refstepcounter{example}\par\medskip
   \noindent\textbf{Example~\theexample. #1} \rmfamily}{\medskip}

\begin{document}
\section{Three examples}
\begin{example}
Create a label in this first example \verb|\label{ex:1}|\label{ex:1}. This is the first example. The \texttt{example} counter will be reset at the start of each new each document \verb|\section|.
\end{example}

\begin{example}
And here's another numbered example. Create a second \verb|\label{ex:2}|\label{ex:2} to later reference this one. In Example \ref{ex:1} we read... something. 
\end{example}

\begin{example}
And here's another numbered example: use \verb|\theexample| to typeset the number currently assigned to the \texttt{example} counter: it is  \theexample.
\end{example}

\section{Another section}
We've just started a new section meaning that the  \texttt{example} counter has been set to \theexample.  
We'll reference examples from the previous section (Examples \ref{ex:1} and \ref{ex:2}).  This is a dummy section with no purpose whatsoever but to contain text. The \texttt{section} counter for this section can be typeset using \verb|\thesection|: it is  currently assigned the value of \thesection.

\begin{example}
This is the first example in this section: the \texttt{example} counter has been stepped and now set to \theexample. 
\end{example}
\end{document}
```

--------------------------------

### Define and Use Custom Example Counter with `\counterwithin*`

Source: https://www.overleaf.com/learn/latex/Counters%23Introduction_to_LaTeX_counters

This example defines a custom `example` environment that is automatically numbered within sections using `\counterwithin*`. It shows how to increment the counter and display it along with the section number.

```latex
\documentclass{article}
\counterwithin{equation}{section}
\newcounter{example}
\counterwithin*{example}{section}
\newenvironment{example}[1][]{% 
\stepcounter{example}%
\par\vspace{5pt}\noindent
\fbox{\textbf{Example~\thesection.\theexample}}%
\hrulefill\par\vspace{10pt}\noindent\rmfamily}% 
{\par\noindent\hrulefill\vrule width10pt height2pt depth2pt\par}
\begin{document}
\section{First equation}
Some introductory text...
\begin{example}
\begin{equation}
    f(x)=\frac{x}{1+x^2}
\end{equation}
\end{example}

\subsection{More detail}
\begin{example}
Here we discuss
\begin{equation}
    f(x)=\frac{x+1}{x-1}
\end{equation}
... but don't say anything
\end{example}

\subsubsection{Even more detail}
\begin{example}
\begin{equation}
    f(x)=\frac{x^2}{1+x^3}
\end{equation}
\begin{equation}
    f(x+\delta x)=\frac{(x+\delta x)^2}{1+(x+\delta x)^3}
\end{equation}
\end{example}

\section{Third equation}
\begin{example}
The following function...
\begin{equation}
    f_1(x)=\frac{x+1}{x-1}
\end{equation}
..is a function
\begin{equation}
    f_2(x)=\frac{x^2}{1+x^3}
\end{equation}
\begin{equation}
    f_3(x)=\frac{3+ x^2}{1-x^3}
\end{equation}
\end{example}
\end{document}
```

--------------------------------

### Example \hbox with Horizontal Glue

Source: https://www.overleaf.com/learn/latex/Articles/How_TeX_Calculates_Glue_Settings_in_an_%5Chbox

Demonstrates the use of \hskip with varying stretch and shrink components within an \hbox. This setup is used to illustrate how TeX calculates the final space between elements.

```tex
\hbox to100pt{\n A\hskip4pt plus3pt minus 2pt%\n B\hskip 0pt plus 2fil% \n C\hskip 0pt plus 2fill%\n D\hskip 0pt plus 3fill%\n}
```

--------------------------------

### Example pTeX Document with latexmkrc

Source: https://www.overleaf.com/learn/latex/Latex-questions/Does_Overleaf_support_pTeX%3F

This snippet shows a complete Overleaf project setup including the main LaTeX file and the `latexmkrc` configuration for pTeX.

```latex
\documentclass{jsarticle}

\bibliographystyle{jplain}
\title{p\LaTeX\ 実験}
\author{林蓮枝}

\begin{document}

\maketitle

\begin{abstract}
本稿では、文書組版システムp\LaTeX{}の使い方を解説します。p\LaTeX{}を利用するときには、あらかじめ文章中に\TeX{}コマンドと呼ばれる組版用の指示を混在させ\ldots
\end{abstract}

\section{導入}
こんにちは世界！

\end{document}
```

```bash
$latex = 'platex';
$bibtex = 'pbibtex';
$dvipdf = 'dvipdfmx %O -o %D %S';
```

--------------------------------

### Define and Use an Example Environment with Referencing

Source: https://www.overleaf.com/learn/latex/Counters%23Introduction_to_LaTeX_counters

Illustrates creating a custom environment 'example' that uses \refstepcounter to increment a counter and enable \label and \ref for cross-referencing. The counter resets with each new section.

```latex
\documentclass{article}

\newcounter{example}[section]
\newenvironment{example}[1][]
{\refstepcounter{example}\par\medskip
   \noindent\textbf{Example~\theexample. #1} \rmfamily}{\medskip}

\begin{document}
\section{Three examples}
\begin{example}
Create a label in this first example \verb|\label{ex:1}|\label{ex:1}. This is the first example. The \texttt{example} counter will be reset at the start of each new each document \verb|\section|.
\end{example}

\begin{example}
And here's another numbered example. Create a second \verb|\label{ex:2}|\label{ex:2} to later reference this one. In Example \ref{ex:1} we read... something. 
\end{example}

\begin{example}
And here's another numbered example: use \verb|\theexample| to typeset the number currently assigned to the \texttt{example} counter: it is  \theexample.
\end{example}

\section{Another section}
We've just started a new section meaning that the  \texttt{example} counter has been set to \theexample.  
We'll reference examples from the previous section (Examples \ref{ex:1} and \ref{ex:2}).  This is a dummy section with no purpose whatsoever but to contain text. The \texttt{section} counter for this section can be typeset using \verb|\thesection|: it is  currently assigned the value of \thesection.

\begin{example}
This is the first example in this section: the \texttt{example} counter has been stepped and now set to \theexample. 
\end{example}
\end{document}
```

--------------------------------

### Minimal xskak Example: Empty and Initial Chessboards

Source: https://www.overleaf.com/learn/latex/Chess_notation

This example demonstrates the basic usage of the `xskak` package to typeset an empty chessboard and a chessboard with pieces in their initial positions. Ensure `xskak` is loaded before using `\chessboard` and `\newchessgame` commands.

```latex
\documentclass{article}
\usepackage{xskak}
\begin{document}
\chessboard[showmover=false]
\newchessgame
\chessboard
The small white square to the right of the second board is called the \textit{mover}.
\end{document}
```

--------------------------------

### LaTeX Example with Itemized List

Source: https://www.overleaf.com/learn/latex/Commands%23Defining_a_new_command

Illustrates how to create a bulleted list in LaTeX using the itemize environment and the \item command, with an example of customizing list markers.

```latex
\documentclass{article}
\begin{document}
A list example:
\begin{itemize}
  \item[\S] First item
  \item Second item
\end{itemize}
\end{document}


```

--------------------------------

### Demonstrate \marginpar with different alignments

Source: https://www.overleaf.com/learn/latex/Margin_notes

This example showcases the \marginpar command with conditional text for left and right pages, and demonstrates \raggedright and \raggedleft for alignment. It requires the geometry and hyperref packages for page setup and URL support.

```latex
\documentclass[twoside]{article}
\usepackage[a4paper, marginparwidth=75pt, total={10cm, 10cm}]{geometry}
\usepackage{hyperref}
\usepackage{marginnote}
\begin{document}
\section{Lorem Ipsum}
\footnote{Source text: Wikipedia (\url{https://en.wikipedia.org/wiki/Lorem_ipsum})}But I must explain to you how all this mistaken idea of reprobating pleasure and extolling pain arose. To do so, I will give you a complete account of the system, and expound the actual teachings of the great explorer of the truth, the master-builder of human happiness. \marginpar[Note 1: text for left-hand side text]{Note 1: text for right-hand side of pages, it is set justified.} No one rejects, dislikes or avoids pleasure itself, because it is pleasure, but because those who do not know how to pursue pleasure rationally encounter consequences that are extremely painful. Nor again is there anyone who loves or pursues or desires to obtain pain of itself, because it is pain, but occasionally circumstances occur in which toil and pain can procure him some great pleasure.  \marginpar[Note 2: text for left-hand side text]{\raggedright Note 2: text for right-hand side of pages, it is not justified, but uses \texttt{\string\raggedright}.} To take a trivial example, which of us ever undertakes laborious physical exercise, except to obtain some advantage from it? But who has any right to find fault with a man who chooses to enjoy a pleasure that has no annoying consequences, or one who avoids a pain that produces no resultant pleasure? [33] On the other hand, we denounce with righteous indignation and dislike men who are so beguiled and demoralized by the charms of pleasure of the moment, so blinded by desire, that they cannot foresee the pain and trouble that are bound to ensue; and equal blame belongs to those who fail in their duty through weakness of will, which is the same as saying through shrinking from toil and pain. These cases are perfectly simple and easy to distinguish. In a free hour, when our power of choice is untrammeled and when nothing prevents our being able to do what we like best, every pleasure is to be welcomed and every pain avoided. \marginpar[\raggedleft Note 3: text for left-hand side of pages, it is not justified, but uses \texttt{\string\raggedleft}]{Note 3: text for left-hand side of pages}But in certain circumstances and owing to the claims of duty or the obligations of business it will frequently occur that pleasures have to be repudiated and annoyances accepted.  The wise man therefore always holds in these matters to this principle of selection: he rejects pleasures to secure other greater pleasures, or else he endures pains to avoid worse pains.
\end{document}
```

--------------------------------

### Defining a Custom Environment with Nested Counters

Source: https://www.overleaf.com/learn/latex/Counters%23Accessing_and_printing_counter_values

Defines a custom 'example' environment that increments a counter and displays it along with the section number. This setup is useful for creating custom-numbered examples within document sections.

```latex
\newenvironment{example}[1][]{%
\stepcounter{example}%
\par\vspace{5pt}\noindent
\fbox{\textbf{Example~\thesection.\theexample}}%
\hrulefill\par\vspace{10pt}\noindent\rmfamily}% 
{\par\noindent\hrulefill\vrule width10pt height2pt depth2pt}
```

--------------------------------

### Calling the Example TeX Macro

Source: https://www.overleaf.com/learn/latex/Articles/What_is_a_TeX_token_list

Demonstrates how to invoke the previously defined `\mymacro` with appropriate arguments to produce output.

```tex
\mymacro abc THIS TEXT defz

```

--------------------------------

### Basic modiagram Environment Setup

Source: https://www.overleaf.com/learn/latex/Molecular_orbital_diagrams

This snippet demonstrates the minimal setup for creating a molecular orbital diagram using the `modiagram` environment. It includes importing the package and defining a single atom with its basic energy sub-levels.

```latex
\documentclass{article}
\usepackage{modiagram}
\begin{document}

First example atoms:

\begin{modiagram}
\atom{left}{1s, 2s, 2p}
\end{modiagram}
\end{document}
```

--------------------------------

### Main file setup with `subfiles` package

Source: https://www.overleaf.com/learn/latex/Multi-file_LaTeX_projects

Include `\usepackage{subfiles}` in the preamble and use `\subfile{}` to import external files. Best loaded last in the preamble.

```latex
\documentclass{article}
\usepackage{graphicx}
\graphicspath{{images/}}

\usepackage{blindtext}

\usepackage{subfiles} % Best loaded last in the preamble

\title{Subfiles package example}
\author{Overleaf}
\date{ }

\begin{document}

\maketitle

\section{Introduction}

\subfile{sections/introduction}

\section{Second section}

\subfile{sections/section2}

\end{document}
```

--------------------------------

### Example Usage of Numbered Environments

Source: https://www.overleaf.com/learn/latex/Environments%23Defining_a_new_environment

Demonstrates using both manually defined and \newtheorem-based numbered environments, showing how numbering resets with sections.

```latex
\documentclass{article}
% Define our numbered environment within the preamble
\newcounter{example}[section]
\newenvironment{example}[1][]
{\refstepcounter{example}\par\medskip
   \noindent \textbf{Example~\theexample. #1} \rmfamily}{\medskip}

% Another numbered environment defined with \newtheorem
\usepackage{amsmath} % For the \newtheorem command
\newtheorem{SampleEnv}{Sample Environment}[section]
\begin{document}

\section{User-defined numbered environments}

\begin{example}
First user-defined numbered environment (number \theexample).
\end{example}

\begin{example}
Second user-defined numbered environment (number \theexample).
\end{example}

\section{More user-defined numbered environments}
Note how the example numbering has restarted at 1:

\begin{example}
First user-defined numbered environment (number \theexample).
\end{example}

\begin{SampleEnv}
User-defined environment created with the \verb|\newtheorem| command.
\end{SampleEnv}
\end{document}
```

--------------------------------

### Example Usage of Boxed Environment

Source: https://www.overleaf.com/learn/latex/Environments%23Defining_a_new_environment

Demonstrates using the 'boxed' environment with and without providing a value for the optional first argument.

```latex
\documentclass{article}
% Note the default value for the first
% argument is provided by [This is a box]
\newenvironment{boxed}[2][This is a box]{\begin{center}
    Argument 1 (\#1)=#1\[1ex]
    \begin{tabular}{|p{0.9\textwidth}|}
    \hline\
    Argument 2 (\#2)=#2\[2ex]
    }
    { 
    \\\\hline
    \end{tabular} 
    \end{center}
    }
\begin{document}
\textbf{Example 1}: Use the default value for the first argument:
 
\begin{boxed}{Some preliminary text}
This text is \textit{inside} the environment.
\end{boxed}

This text is \textit{outside} the environment.

\vskip12pt

\textbf{Example 2}: Provide a value for the first argument:
 
\begin{boxed}[This is not the default value]{Some more preliminary text}
This text is still \textit{inside} the environment.
\end{boxed}

This text is also \textit{outside} the environment.
\end{document}
```

--------------------------------

### Minimal Beamer Presentation Example

Source: https://www.overleaf.com/learn/latex/Beamer

A basic Beamer document structure to create a title page and a content frame. Use this as a starting point for simple presentations.

```latex
\documentclass{beamer}
%Information to be included in the title page:
\title{Sample title}
\author{Anonymous}
\institute{Overleaf}
\date{2021}

\begin{document}

\frame{\titlepage}

\begin{frame}
\frametitle{Sample frame title}
This is some text in the first frame. This is some text in the first frame. This is some text in the first frame.
\end{frame}

\end{document}

```

--------------------------------

### Basic Math Mode Spacing Example

Source: https://www.overleaf.com/learn/latex/Spacing_in_math_mode

Demonstrates the basic structure for using math mode in LaTeX. This example shows how to define sets with mathematical notation.

```latex
\documentclass{article}
\usepackage{amssymb}
\begin{document}
Assume we have the next sets
\[
S = \{ z \in \mathbb{C}\, |\, |z| < 1 \} \quad \textrm{and} \quad S_2=\partial{S}
\]
\end{document}
```

--------------------------------

### Example: todonotes Package Usage

Source: https://www.overleaf.com/learn/latex/Questions/Can_I_add_inline_or_margin_comments_to_the_pdf%3F

A comprehensive example demonstrating various todonotes features, including different note styles, colors, font sizes, and inline comments. It also shows how to exclude notes from the list.

```latex
\documentclass[a4paper]{article}
\usepackage[colorlinks]{hyperref}
\usepackage[colorinlistoftodos]{todonotes}
\begin{document}
\listoftodos[A list of things I need to finish]

Lorem ipsum dolor sit amet, consectetuer adipiscing elit. Nulla
\todo{Plain todonotes.} urna. Maecenas interdum nunc in augue. 
Mauris quis massa in ante tincidunt mollis. Proin imperdiet. 
Donec porttitor pede id est. Sed in ante. Integer id arcu. Nam 
lectus nisl, posuere sit amet, imperdiet ut, tristique ac, lorem. 
In erat. In commodo enim. \todo[color=blue!40]{Todonote with a different color.}%
Phasellus libero ipsum, tempor a, pharetra consequat, pellentesque
sit amet, sem. Praesent ut augue luctus elit adipiscing ultricies.
Vestibulum suscipit cursus leo. Nullam molestie justo.

Morbi dui. Morbi convallis mi sed sem. Nulla convallis lacus vitae
risus. Phasellus adipiscing. Nullam tortor. Sed laoreet aliquam
ante. Vestibulum diam. Pellentesque nec leo. Pellentesque velit.
\todo[nolist]{Todonote that is only shown in the margin and not in
the list of todos.}%
Praesent congue mi eu ipsum cursus fringilla. Etiam leo erat,
tristique et, pharetra eget, mollis vitae, velit. In hac habitasse
\todo[size=\small, color=green!40]{A note with a small fontsize.}%
platea dictumst. In quam nibh, facilisis et, laoreet non, facilisis
tempus, justo. Class aptent taciti sociosqu ad litora torquent per
conubia nostra, per inceptos himenaeos.

\todo[inline]{testing testing}

Donec nulla lectus, faucibus sit amet, auctor non, consectetuer
quis, pede. Nullam dictum. Nullam suscipit, ligula in scelerisque
\todo[noline]{A note with no line back to the text.}%
posuere, sapien purus rutrum magna, vitae pharetra leo quam vel
tortor. Donec eleifend condimentum sapien. Etiam sed orci. Aliquam
\todo[inline, color=red!50]{Inline todonotes.}%
tempor. Pellentesque egestas tortor id eros. Donec mauris justo,
commodo id, pellentesque id, eleifend non, mi. Duis venenatis
sagittis metus. Donec tempus metus id lacus. Praesent vel diam.
Morbi nec ante. Vestibulum varius felis ac lacus. Nulla vitae neque
\todo[inline, color=green!40]{A note with no line back to the text.}%
in nibh bibendum volutpat. Quisque accumsan diam. Aenean ultricies
nisl ac lacus. Aliquam posuere. Aenean venenatis tortor in felis.
\end{document}
```

--------------------------------

### Minimal biblatex Example

Source: https://www.overleaf.com/learn/latex/Bibliography_management_with_biblatex

A basic LaTeX document structure demonstrating the minimal setup for using the biblatex package to cite and print a bibliography.

```latex
\documentclass{article}
\usepackage[utf8]{inputenc}
\usepackage[english]{babel}

\usepackage{biblatex}
\addbibresource{sample.bib}

\begin{document}
Let's cite! The Einstein's journal paper \cite{einstein} and the Dirac's 
book \cite{dirac} are physics related items. 

\printbibliography

\end{document>

```

--------------------------------

### Basic Nomenclature Example

Source: https://www.overleaf.com/learn/latex/Nomenclatures

This is a fundamental example demonstrating how to use the `nomencl` package to create a simple list of symbols and their descriptions.

```latex
\documentclass{article}
\usepackage{nomencl}
\makenomenclature

\begin{document}
Here is an example:
\nomenclature{\(c\)}{Speed of light in a vacuum}
\nomenclature{\(h\)}{Planck constant}

\printnomenclature
\end{document}
```

--------------------------------

### Full Set of Text Formatting Examples in LaTeX

Source: https://www.overleaf.com/learn/latex/Bold%2C_italics_and_underlining

A collection of various text formatting examples including bold, italics, underline, and emphasis, demonstrating their usage and nesting.

```latex
First example, bold, italics and underline:

Some of the \\textbf{greatest} discoveries in \\underline{science} were made by \\textbf{\\\\emph{accidentப்பட்டன}}.

\\vspace{1.5cm}

Example of italicized text: 

Some of the greatest discoveries in science were made by \\emph{accident}.

\\vspace{1.5cm}

Example of boldface text:

Some of the \\textbf{greatest} discoveries in science were made by accident.

\\vspace{1.5cm}

Example of underlined text:

Some of the greatest discoveries in \\underline{science} were made by accident.

\\vspace{1.5cm}

Example of emphasized text in different contexts:

Some of the greatest \\emph{discoveries} in science were made by accident.

\\textit{Some of the greatest \\emph{discoveries} in science were made by accident.}

\\textbf{Some of the greatest \\emph{discoveries} in science were made by accident.}
```

--------------------------------

### Beamerposter Preamble Setup

Source: https://www.overleaf.com/learn/latex/Posters

Configure the document class, load necessary packages, set the theme, and define poster dimensions using beamerposter options. This setup is essential for creating a scientific poster.

```latex
\documentclass{beamer}
  \usepackage{times}
  \usepackage{amsmath,amsthm, amssymb}
  \boldmath
  \usetheme{RedLion}
  \usepackage[orientation=portrait,size=a0,scale=1.4]{beamerposter}

  \title[Beamer Poster]{Overleaf example of the beamerposter class}
  \author[welcome@overleaf.com]{Overleaf Team}
  \institute[Overleaf University]
  {The Overleaf institute, Learn faculty}
  \date{\today}
  
  \logo{\includegraphics[height=7.5cm]{overleaf-logo}}


```

```latex
usepackage[orientation=portrait,size=a0,scale=1.4]{beamerposter}


```

--------------------------------

### Define Custom Example Environment

Source: https://www.overleaf.com/learn/latex/Counters%23Introduction_to_LaTeX_counters

Defines a custom LaTeX environment named 'example'. It increments a counter and formats the output with a box containing 'Example' followed by section and example numbers, separated by a horizontal rule.

```latex
\newenvironment{example}[1][]{% 
\stepcounter{example}%
\par\vspace{5pt}\noindent
\fbox{\textbf{Example~\thesection.\theexample}}%
\hrulefill\par\vspace{10pt}\noindent\rmfamily}% 
{\par\noindent\hrulefill\vrule width10pt height2pt depth2pt}
```

--------------------------------

### Overleaf Example: Exploring Math Display Styles

Source: https://www.overleaf.com/learn/latex/Display_style_in_math_mode

This is a complete LaTeX document example that can be opened in Overleaf. It demonstrates inline and display math, and the effects of \textstyle, \scriptstyle, and \scriptscriptstyle within an \begin{align*} environment.

```latex
\documentclass{article}
\usepackage{amsmath}
\title{Exploring math display styles}
\author{Overleaf team}
\begin{document}
\maketitle
Depending on the value of \(x\) the equation \( f(x) = \sum_{i=0}^{n} \frac{a_i}{1+x} \) may diverge or converge.

\displaymath f(x) = \sum_{i=0}^{n} \frac{a_i}{1+x} \]

\vspace{1cm}

Inline maths elements can be set with a different style: \(f(x) = \displaystyle \frac{1}{1+x}\). The same is true for display math material:

\begin{align*}
f(x) = \sum_{i=0}^{n} \frac{a_i}{1+x} \\
\textstyle f(x) = \sum_{i=0}^{n} \frac{a_i}{1+x} \\
\scriptstyle f(x) = \sum_{i=0}^{n} \frac{a_i}{1+x} \\
\scriptscriptstyle f(x) = \sum_{i=0}^{n} \frac{a_i}{1+x}
\end{align*}
\end{document}

```

--------------------------------

### Markdown to HTML Conversion Example

Source: https://www.overleaf.com/learn/latex/Articles/How_to_write_in_Markdown_on_Overleaf

Shows the resulting HTML output after converting the example Markdown text. This illustrates the transformation process.

```html
<h1 id="grocerylist">Grocery list</h1>
<p><em>Remember</em> to grab as much as we can during upcoming <a href="http://acme-marg.com">sales</a>!</p>
<h2 id="food">Food</h2>
<ul>
<li>baked beans</li>
<li>spaghetti</li>
</ul>
<h2 id="stationery">Stationery</h2>
<ul>
<li>writing pad</li>
<li>pencils</li>
</ul>

```

--------------------------------

### Example of 'blank' pages in a book class document

Source: https://www.overleaf.com/learn/latex/Headers_and_footers

This example demonstrates how LaTeX inserts a 'blank' page (page 2) to ensure the next chapter starts on a right-hand page when using the `book` class without `openany`. This page may still contain headers.

```latex
\begin{document}

\frontmatter
\maketitle
This is frontmatter which uses Roman numerals.

\mainmatter
\chapter{Where do I start}
Chapter 1: A short chapter that ends on page 1.

\chapter{Things I remember}
Chapter 2: Starts on page 3, so \LaTeX{} inserts a ``blank'' page 2.

\end{document}


```

--------------------------------

### Combining \offinterlineskip with Non-zero \lineskip

Source: https://www.overleaf.com/learn/latex/Articles/How_to_change_paragraph_spacing_in_LaTeX

This LaTeX example demonstrates how to use \offinterlineskip with a non-zero \lineskip value to achieve specific line spacing effects. It includes document setup, geometry package for page size, and inputting a text file with custom macros.

```latex
\documentclass{article}
\title{Demonstrating non-zero lineskip}
% Choose a conveniently small page size
\usepackage[paperheight=16cm,paperwidth=12cm,textwidth=8cm]{geometry}
\begin{document}

\offinterlineskip % This sets \lineskip to 0pt

\setlength{\lineskip}{5pt} % We want non-zero \lineskip

\input text.tex % A generated TeX file which defines the macros
% \mytextA and \mytextB, each of which typesets a paragraph

\mytextA % Typeset a paragraph

% Now change \lineskip to a flexible glue value and typeset
% another paragraph

\setlength{\lineskip}{5pt plus5pt minus5pt}

\mytextA

\end{document}


```

--------------------------------

### Basic Multicolumn Layout

Source: https://www.overleaf.com/learn/latex/Multiple_columns%23Inserting_vertical_rulers

This example shows how to set up a basic three-column layout using the `multicols` environment. The optional header text can include sections and paragraphs.

```latex
\documentclass{article}
\usepackage{blindtext}
\usepackage{multicol}
\title{Multicols Demo}
\author{Overleaf}
\date{April 2021}

\begin{document}
\maketitle

\begin{multicols}{3}
[
\section{First Section}
All human things are subject to decay. And when fate summons, Monarchs must obey.
]
\blindtext\blindtext
\end{multicols}

\end{document}
```

--------------------------------

### LaTeX: Full Document Example

Source: https://www.overleaf.com/learn/latex/Font_sizes%2C_families%2C_and_styles

A comprehensive example combining various font size, family, and style commands within a complete LaTeX document structure.

```latex
\documentclass{article}
\begin{document}
%Example of different font sizes and types
This is a simple example, {\tiny this will show different font sizes} and also \textsc{different font styles}.

\vspace{1cm}

%Example of different font sizes and types
In this example the {\huge huge font size} is set and the {\footnotesize Foot note size also}. There's a fairly large set of font sizes.

\vspace{1cm}

%Example of different font sizes and types
In this example, a command and a switch are used. \texttt{A command is used to change the style of a sentence}.

\sffamily
A switch changes the style from this point to the end of the document unless another switch is used.
\rmfamily

\vspace{1cm}

%Example of different font sizes and types
Part of this text is written \textsl{in different font style} to highlight it.
\end{document}

```

--------------------------------

### Build ktex.web with multiple change files and view TIE output

Source: https://www.overleaf.com/learn/latex/How_Overleaf_created_the_TeX_primitive_reference_data

This example demonstrates applying a series of change files to `tex.web` to generate `ktex.web`, a composite WEB file suitable for the Web2C process. It includes the TIE command and its typical output, showing the files being processed and progress indicators.

```bash
tie -m ktex.web tex.web tex.ch enctex.ch synctex-def.ch0 synctex-mem.ch0 synctex-mem.ch2 synctex-rec.ch0 synctex-rec.ch1 synctex-rec.ch2 tex-binpool.ch
This is TIE, CWEB Version 2.4.
Copyright (c) 1989,1992 by THD/ITI. All rights reserved.
(tex.web)
(tex.ch)
(enctex.ch)
(synctex-def.ch0)
(synctex-mem.ch0)
(synctex-mem.ch2)
(synctex-rec.ch0)
(synctex-rec.ch1)
(synctex-rec.ch2)
(tex-binpool.ch)
....500....1000....1500....2000....2500....3000....3500....4000....4500
....5000....5500....6000....6500....7000....7500....8000....8500....9000
....9500....10000....10500....11000....11500....12000....12500....13000
....13500....14000....14500....15000....15500....16000....16500....17000
....17500....18000....18500....19000....19500....20000....20500....21000
....21500....22000....22500....23000....23500....24000....24500....
(No errors were found.)

```

--------------------------------

### Full Spanish LaTeX Document Example

Source: https://www.overleaf.com/learn/latex/Spanish

A complete example demonstrating `fontenc` and `babel` for Spanish, including localized mathematical commands and quotation marks.

```latex
\documentclass{article}
\usepackage[T1]{fontenc}
\usepackage[spanish]{babel}
\begin{document}
\section{Sección con teoremas}
Esta sección es para ver que pasa con los comandos que definen texto

\[ \lim x =  \tg {\theta} + \max \{3.52, 4.22 \} \]

El paquete también agrega un comportamiento especial a <<éstas márcas para hacer citas textuales>> tal como lo indican las relgas de la RAE.
\end{document}
```

--------------------------------

### Example Input for \expandafter Analysis

Source: https://www.overleaf.com/learn/latex/Articles/How_does_%5Cexpandafter_work%3A_A_detailed_study_of_consecutive_%5Cexpandafter_commands

Defines the input sequence for analyzing the behavior of consecutive \expandafter commands.

```latex
\expandafter1\expandafter2\expandafter3 TXTY
```

--------------------------------

### Complete LaTeX Document Example

Source: https://www.overleaf.com/learn/latex/Integrals%2C_sums_and_limits

A full LaTeX document demonstrating integrals, multiple integrals, sums, products, and limits, including necessary packages and sections.

```latex
\documentclass{article}
\title{Integrals, Sums and Limits}
\author{Overleaf}
\date{}
\usepackage{amsmath}

\begin{document}

\maketitle

\section{Integrals}

Integral \(\int_{a}^{b} x^2 dx\) inside text.

\medskip

The same integral on display:
\[
    \int_{a}^{b} x^2 \,dx
\]
and multiple integrals:
\begin{gather*}
    \iint_V \mu(u,v) \,du\,dv
\
    \iiint_V \mu(u,v,w) \,du\,dv\,dw
\
    \iiiint_V \mu(t,u,v,w) \,dt\,du\,dv\,dw
\
    \idotsint_V \mu(u_1,\dots,u_k) \,du_1 \dots du_k
\
    \oint_V f(s) \,ds
\end{gather*}

\section{Sums and products}

Sum \(\sum_{n=1}^{\infty} 2^{-n} = 1\) inside text.

The same sum on display:
\[
    \sum_{n=1}^{\infty} 2^{-n} = 1
\]

Product \(\prod_{i=a}^{b} f(i)\) inside text.

The same product on display:
\[
    \prod_{i=a}^{b} f(i)
\]

\section{Limits}

Limit \(\lim_{x\to\infty} f(x)\) inside text.

The same limit on display:
\[
    \lim_{x\to\infty} f(x)
\]

\end{document}
```

--------------------------------

### Define Custom Environment for Examples

Source: https://www.overleaf.com/learn/latex/Counters

Defines a custom 'example' environment that increments a counter and formats the output with a boxed title including section and example numbers. This environment is typically used within a document that employs \counterwithin or \counterwithout.

```latex
\newenvironment{example}[1][]{\stepcounter{example}%\par\vspace{5pt}\noindent
\fbox{\textbf{Example~\thesection.\theexample}}%\hrulefill\par\vspace{10pt}\noindent\rmfamily}{\par\noindent\hrulefill\vrule width10pt height2pt depth2pt\par}
```

--------------------------------

### Create Proof Environments with amsthm

Source: https://www.overleaf.com/learn/latex/Theorems_and_proofs

Utilize the 'proof' environment provided by the amsthm package to visually distinguish mathematical proofs from regular text. This example also shows defining a 'lemma' environment.

```latex
\documentclass{article}
\usepackage[english]{babel}
\usepackage{amsthm}

\newtheorem{theorem}{Theorem}[section]
\newtheorem{lemma}[theorem]{Lemma}

\begin{document}
\section{Introduction}
\begin{lemma}
Given two line segments whose lengths are \(a\) and \(b\) respectively there 
is a real number \(r\) such that \(b=ra\).
\end{lemma}

\begin{proof}
To prove it by contradiction try and assume that the statement is false,
proceed from there and at some point you will arrive to a contradiction.
\end{proof}
\end{document}
```

--------------------------------

### Infinite Glue Example

Source: https://www.overleaf.com/learn/latex/Articles/How_TeX_Calculates_Glue_Settings_in_an_%5Chbox

Demonstrates the use of 'infinite' stretch and shrink components using fil and fill units within \hskip. This allows glue to stretch or shrink by any desired amount.

```tex
\hskip 3pt plus 2fil minus 1fill
```

--------------------------------

### Include a LaTeX file using \include

Source: https://www.overleaf.com/learn/latex/Management_in_a_large_project%23Inputting_and_including_files

Use \include{filename} to insert the content of filename.tex. The file should not contain LaTeX preamble code. LaTeX will start a new page before processing the inputted material. Each \include'd file gets its own .aux file.

```latex
\include{filename}
```

--------------------------------

### All Examples in One Project

Source: https://www.overleaf.com/learn/latex/Subscripts_and_superscripts

This code block aggregates several examples of subscript and superscript usage, including integrals, combined superscripts/subscripts, long expressions, nested structures, summations, products, and square roots.

```latex
Here are some examples of simple usage of subscripts and superscripts:

\[ 
  \int\limits_0^1 x^2 + y^2 \ dx 
\]

\vspace{1cm}

Using superscript and subscripts in the same expression

\[ 
  a_1^2 + a_2^2 = a_3^2 
\]

\vspace{1cm}

Longer subscripts and superscripts:

\[ 
  x^{2 \alpha} - 1 = y_{ij} + y_{ij}  
\]

\vspace{1cm}

Nested subscripts and superscripts

\[ 
  (a^n)^{r+s} = a^{nr+ns} 
\]

\vspace{1cm}

Example of a mathematical equation with subscripts and superscripts

\[ 
  \sum_{i=1}^{\infty} \frac{1}{n^s} = \prod_p \frac{1}{1 - p^{-s}} 
\]

\vspace{1cm}

Squared root usage

\[ 
  \sqrt[4]{4ac} = \sqrt{4ac}\sqrt{4ac} 
\]
```

--------------------------------

### Product with Limits (Reference)

Source: https://www.overleaf.com/learn/latex/Subscripts_and_superscripts

Reference example for a product operator with specified lower and upper bounds.

```latex
`\prod_{i=1}^n`
```

--------------------------------

### Integral with Limits (Reference)

Source: https://www.overleaf.com/learn/latex/Subscripts_and_superscripts

Reference example for an integral with specified lower and upper limits.

```latex
`\int_{i=1}^n`
```

--------------------------------

### Incorrect \verb command with mismatched delimiters

Source: https://www.overleaf.com/learn/latex/Errors/LaTeX_Error%3A_%5Cverb_ended_by_end_of_line

This example shows incorrect usage of the \verb command where the opening delimiter is omitted or mismatched, leading to errors. Always ensure the start and end delimiters are identical and present.

```latex
\verb{\frac{1}{2}}
```

```latex
\verb \frac{1}{2}
```

```latex
{\verb \frac{1}{2}}
```

```latex
\verb!\frac{1}{2}
```

--------------------------------

### Complete LaTeX Document with Font Examples

Source: https://www.overleaf.com/learn/latex/Font_sizes_and_kinds%23Reference_guide

A full LaTeX document example combining various font size, family, and style commands demonstrated in the preceding sections.

```latex
\documentclass{article}
\begin{document}
%Example of different font sizes and types
This is a simple example, {\tiny this will show different font sizes} and also \textsc{different font styles}. 

\vspace{1cm}

%Example of different font sizes and types
In this example the {\huge huge font size} is set and the {\footnotesize Foot note size also}. There's a fairly large set of font sizes.

\vspace{1cm}

%Example of different font sizes and types
In this example, a command and a switch are used. \texttt{A command is used to change the style of a sentence}.

\sffamily
A switch changes the style from this point to the end of the document unless another switch is used.
\rmfamily

\vspace{1cm}

%Example of different font sizes and types
Part of this text is written \textsl{in different font style} to highlight it.
\end{document}
```

--------------------------------

### Use a Custom LaTeX Package

Source: https://www.overleaf.com/learn/latex/Writing_your_own_package

This document demonstrates how to use the custom 'examplepackage' with the 'red' option. It includes the necessary \documentclass, \usepackage, and \makeindex commands, and shows how to use the custom 'example' environment and '\important' command.

```tex
\documentclass{article}
\usepackage[utf8]{inputenc}

\usepackage[red]{examplepackage}

\makeindex

\title{Package Example}
\author{Team Learn ShareLaTeX}
\date{ }

\begin{document}

\maketitle

\section{Introduction}
In this document a new package is tested. This package allows special numbered
environments

\begin{example}
This text is inside a special environment, some boldface text is printed
at the beginning and a new indentation is set.
\end{example}

Also, there's a special command for \important{important!words} that will be
printed in a special \important{colour} depending on the parameter used in the
\important{package}
importation statement. Because it's \important{important}.

\printindex

\end{document}
```

--------------------------------

### Example \hbox with Stretchable Glue

Source: https://www.overleaf.com/learn/latex/Articles/How_TeX_Calculates_Glue_Settings_in_an_%5Chbox

This \hbox demonstrates the use of \hskip with varying stretch components (pt, fil, fill, filll). It serves as the basis for calculating glue settings.

```tex
\hbox to100pt{
A\hskip4pt plus3pt minus 2pt% 
B\hskip 0pt plus 2fil% 
C\hskip 0pt plus 2fill% 
D\hskip 0pt plus 3fill% 
}
```

--------------------------------

### Complete LaTeX Example with Markdown Image Inclusion

Source: https://www.overleaf.com/learn/latex/Articles/How_to_write_in_Markdown_on_Overleaf

A full LaTeX document example showcasing how to include an image using Markdown syntax, with specific package configurations and image settings.

```latex
\documentclass{article}
\usepackage[hybrid]{markdown}
% The mwe package provides example images. Loading it is
% not essential because those images are in LaTeX's search path. 
% Here, we load it for clarity in this example.
\usepackage{mwe}
\begin{document}
\begin{markdown}
This example shows how to import a graphics file. Here we are using an
example image provided by the `mwe` package.

% Use \setkeys{Gin} if you need to change an image's display size

\setkeys{Gin}{width=.5\linewidth}
![This is alt text to describe my image.](example-image.jpg "An example image provided by the \texttt{mwe} package.")
\end{markdown}
\end{document}
```

--------------------------------

### Basic Hebrew Document Setup

Source: https://www.overleaf.com/learn/latex/Multilingual_typesetting_on_Overleaf_using_polyglossia_and_fontspec

A minimal LaTeX document setup for Hebrew, including setting the default language to Hebrew and defining a Hebrew font.

```latex
\documentclass{article}
\usepackage{polyglossia}
\setdefaultlanguage[numerals=hebrew]{hebrew}
\setotherlanguage{english}
\newfontfamily\hebrewfont[Script=Hebrew]{Hadasim CLM}
\begin{document}
\section{מבוא}
זוהי עובדה מבוססות שדעתו של הקורא תהיה מוסחת עלידי טקטס קריא כאשר הוא יביט בפריסתו.  -
```

--------------------------------

### Complete Feynman Diagram Example

Source: https://www.overleaf.com/learn/latex/Feynman_diagrams

A minimal LaTeX document demonstrating the creation of a simple Feynman diagram using `feynmp-auto`, `fmffile`, and `fmfgraph`.

```latex
\documentclass{article}
\usepackage{feynmp-auto}
\begin{document}
\begin{fmffile}{first-diagram}
 \begin{fmfgraph}(120,80)
   \fmfleft{i1,i2}
   \fmfright{o1,o2}
   \fmf{fermion}{i1,v1,o1}
   \fmf{fermion}{i2,v2,o2}
   \fmf{photon}{v1,v2}
 \end{fmfgraph}
\end{fmffile}
\end{document}
```

--------------------------------

### Example with French Quotation Marks using dirtytalk

Source: https://www.overleaf.com/learn/latex/Typesetting_quotations

An example demonstrating the `dirtytalk` package with French quotation marks, including nested quotes.

```latex
\documentclass{article}
\usepackage[french]{babel}
\usepackage[T1]{fontenc}

\usepackage[
    left = \flqq{},
    right = \frqq{},
    leftsub = \flq{},
    rightsub = \frq{}
]{dirtytalk}

\begin{document}
\section{Introduction}

Typing quotations with this package is quite easy:

\say{Here, a quotation is written and even some \say{nested} quotations are possible}
\end{document}
```

--------------------------------

### Example TeX Macro Definition

Source: https://www.overleaf.com/learn/latex/Articles/What_is_a_TeX_token_list

A concrete example of defining a TeX macro named `\mymacro` with specific delimiters and a parameter.

```tex
\def\mymacro abc #1 defz{I typed "#1"!}

```

--------------------------------

### Display Math Mode Examples in LaTeX

Source: https://www.overleaf.com/learn/latex/Mathematical_expressions

Shows how to use display math mode in LaTeX with \[...\] and the \begin{equation}...
\end{equation} environment for numbered equations.

```latex
\documentclass{article}
\begin{document}
The mass-energy equivalence is described by the famous equation

\[E=mc^2\]

discovered in 1905 by Albert Einstein. 
In natural units ($c$ = 1), the formula expresses the identity

\begin{equation}
E=m
\end{equation}
\end{document}

```

--------------------------------

### LaTeX Book Document Class Example

Source: https://www.overleaf.com/learn/latex/Sections_and_chapters

An example using the `book` document class, which supports parts, chapters, sections, subsections, and sub-subsections. Note that `	exttt{\subsubsection}` is not numbered by default.

```latex
\documentclass{book}
\title{Sections and Chapters}
\author{Overleaf}
\date{\today}
\begin{document}
\maketitle
\tableofcontents
\part{History of Lua\TeX}

\chapter{An Introduction to Lua\TeX}

\section{What is it—and what makes it so different?}
Lua\TeX{} is a \textit{toolkit}—it contains sophisticated software tools and components with which you can construct (typeset) a wide range of documents. The sub-title of this article also poses two questions about Lua\TeX: What is it—and what makes it so different? The answer to “What is it?” may seem obvious: “It’s a \TeX{} typesetting engine!” Indeed it is, but a broader view, and one to which this author subscribes, is that Lua\TeX{} is an extremely versatile \TeX-based document construction and engineering system.

\subsection{Explaining Lua\TeX: Where to start?}
The goal of this first article on Lua\TeX{} is to offer a context for understanding what this TeX engine provides and why/how its design enables users to build/design/create a wide range of solutions to complex typesetting and design problems—perhaps also offering some degree of “future proofing” 

\chapter{Lua\TeX: Background and history}
\section{Introduction}
Lua\TeX{} is, in \TeX{} terms, “the new kid on the block” despite having been in active development for over 10 years.

\subsection{Lua\TeX: Opening up \TeX’s “black box”}
Knuth’s original \TeX{} program is the common ancestor of all modern \TeX{} engines in use today and Lua\TeX{} is, in effect, the latest evolutionary step: derived from the pdf\TeX{} program but with the addition of some powerful software components which bring a great deal of extra functionality.

\subsubsection{How Lua\TeX{} processes \texttt{\string\directlua}: A first look}
The ⟨code⟩ provided to \verb|\directlua{<code>}| is first converted to tokens using the processes and calculations discussed above; that sequence of tokens is stored in a token list.
\end{document}
```

--------------------------------

### Constructing a Fixed-Width Horizontal Box with Flexible Glue

Source: https://www.overleaf.com/learn/latex/Articles/Pandora%E2%80%99s_%5Chbox%3A_Using_LuaTeX_to_Lift_the_Lid_of_TeX_Boxes

This example demonstrates creating a 100pt wide \hbox. It uses \hskip with flexible 'plus' and 'fill' components to distribute space and ensure the box reaches the target width.

```tex
\hbox to100pt{
A\hskip4pt plus3pt minus 2pt B% 
\hskip 0pt plus 2fil C% 
\hskip 0pt plus 2fill D% 
\hskip 0pt plus 3fill}
```

--------------------------------

### Use Custom LaTeX Environment with Default and Provided Arguments

Source: https://www.overleaf.com/learn/latex/Environments

Demonstrates using the custom 'boxed' environment. The first example uses the default value for the optional argument, while the second provides a custom value.

```latex
\documentclass{article}
% Note the default value for the first
% argument is provided by [This is a box]
\newenvironment{boxed}[2][This is a box]{\begin{center}
    Argument 1 (\#1)=#1\[1ex]
    \begin{tabular}{|p{0.9\textwidth}|}
    \hline\
    Argument 2 (\#2)=#2\[2ex]
    }
    { 
    \\\hline
    \end{tabular} 
    \end{center}
    }
\begin{document}
\textbf{Example 1}: Use the default value for the first argument:
 
\begin{boxed}{Some preliminary text}
This text is \textit{inside} the environment.
\end{boxed}

This text is \textit{outside} the environment.

\vskip12pt

\textbf{Example 2}: Provide a value for the first argument: 
 
\begin{boxed}[This is not the default value]{Some more preliminary text}
This text is still \textit{inside} the environment.
\end{boxed}

This text is also \textit{outside} the environment.
\end{document}
```

--------------------------------

### Display equation example

Source: https://www.overleaf.com/learn/latex/%5Cabovedisplayskip_and_related_commands

A basic display math equation using the \\ [ and \\ ] delimiters.

```latex
\[ rac{\hbar^2}{2m}\nabla^2\Psi + V(\mathbf{r})\Psi
= -i\hbar \frac{\partial\Psi}{\partial t} \]
```

--------------------------------

### Incorrect Display Math Termination with Single $

Source: https://www.overleaf.com/learn/latex/Errors/Display_math_should_end_with_%24%24

This example shows how starting display math with '$$' and ending with a single '$' causes the 'Display math should end with $$' error.

```latex
\documentclass{article}
\usepackage[textwidth=8cm]{geometry}
\begin{document}
\noindent \verb|$$ E=mc^2$| generates an error because the math is 
started by \texttt{\$\\$} but terminated by a single \texttt{\$}:

$$ E=mc^2$

\noindent\verb|$$ E=mc^2$ $| also generates an error because of the space between
the terminating \texttt{\$} characters:

$$ E=mc^2$ $
\end{document}

```

--------------------------------

### Nested \hbox Example

Source: https://www.overleaf.com/learn/latex/Articles/How_TeX_Calculates_Glue_Settings_in_an_%5Chbox

Illustrates a nested \hbox structure where an inner \hbox is contained within an outer \hbox. This example highlights that glue settings like glue_set, glue_sign, and glue_order are local to their respective boxes.

```tex
\hbox to 75pt{\hfill ABC\hbox to15pt{\hfill D}}
```

--------------------------------

### Enable SyncTeX with compressed output

Source: https://www.overleaf.com/learn/latex/MLTeX_SyncTeX_and_EncTeX_TeX_extensions

Use the -synctex=1 option to enable SyncTeX and generate a compressed .synctex.gz output file.

```bash
pdftex -synctex=1 myfile.tex

```

--------------------------------

### Basic LaTeX Exam Document Setup

Source: https://www.overleaf.com/learn/latex/Typesetting_exams_in_LaTeX

This is the fundamental structure for an exam document using the exam.cls class. It includes essential commands for document setup and question formatting.

```latex
\documentclass{exam}

```

```latex
\documentclass{exam}

\begin{document}

\begin{center}
\fbox{\fbox{\parbox{5.5in}{\centering
Answer the questions in the spaces provided. If you run out of room
for an answer, continue on the back of the page.}}}
\end{center}

\vspace{5mm}
\makebox[0.75\textwidth]{Name and section:\enspace\hrulefill}

\vspace{5mm}
\makebox[0.75\textwidth]{Instructor’s name:\enspace\hrulefill}

\begin{questions}
\question Is it true that \(x^n + y^n = z^n\) if \(x,y,z\) and \(n\) are
positive integers?. Explain.

\question Prove that the real part of all non-trivial zeros of the function
\(\zeta(z)\) is \(\frac{1}{2}\)

\question Compute \[\]int_{0}^{\infty} \frac{\sin(x)}{x}\]
\end{questions}
\end{document}

```

--------------------------------

### Correct verbatim text with \texttt{verbatim} environment (fraction example)

Source: https://www.overleaf.com/learn/latex/Errors/LaTeX_Error%3A_%5Cverb_ended_by_end_of_line

This demonstrates using the \texttt{verbatim} environment to correctly display mathematical fractions or other code-like text that might otherwise cause issues with \verb commands.

```latex
\begin{verbatim}
\frac{1}{2}
\end{verbatim}
```

--------------------------------

### Example Plain Text Bibliography

Source: https://www.overleaf.com/learn/latex/Bibliography_management_with_bibtex

This is an example of a plain text bibliography that can be converted into a structured BibTeX .bib file using tools like text2bib or Edifix.

```plaintext
[1] J. Smith, J. Doe and F. Bar (2001) A ground-breaking study.
Journal of Amazing Research 5(11), pp. 29-34.

[2] ...

```

--------------------------------

### Example TeX Macro Definition

Source: https://www.overleaf.com/learn/latex/How_TeX_macros_actually_work%3A_Part_5

A concrete example of a TeX macro definition used to illustrate the concepts of parameter text and replacement text.

```tex
\def\foo A#1\fake{123 #1}

```

--------------------------------

### LaTeX Example of Undefined References

Source: https://www.overleaf.com/learn/latex/Errors/There_were_undefined_references

This example demonstrates how typos in \ref{...} or referencing undefined labels lead to 'undefined references' errors in LaTeX.

```latex
\section{introduction}\label{introduction}
A typo when referencing the introduction would be \ref{intorduction}.

Another error is referencing a label which has never been defined such as \ref{section1}

```

--------------------------------

### LuaTeX: Printing 'Hello, World!' with \directlua

Source: https://www.overleaf.com/learn/latex/Articles/An_Introduction_to_LuaTeX_%28Part_2%29%3A_Understanding_%5Cdirectlua

A basic example of using \directlua to execute Lua code within LuaTeX, specifically calling tex.print to typeset a string.

```tex
\directlua{tex.print("Hello, World!")}
```

--------------------------------

### BibTeX Entry Examples

Source: https://www.overleaf.com/learn/latex/Bibliography_management_in_LaTeX

Examples of common BibTeX entry types like article, book, online, and inbook, demonstrating the standard syntax for storing bibliographic information.

```bibtex
@article{einstein,
    author = "Albert Einstein",
    title = "{Zur Elektrodynamik bewegter K{"o}rper}. ({German})
    [{On} the electrodynamics of moving bodies]",
    journal = "Annalen der Physik",
    volume = "322",
    number = "10",
    pages = "891--921",
    year = "1905",
    DOI = "http://dx.doi.org/10.1002/andp.19053221004",
    keywords = "physics"
}

@book{dirac,
    title = {The Principles of Quantum Mechanics},
    author = {Paul Adrien Maurice Dirac},
    isbn = {9780198520115},
    series = {International series of monographs on physics},
    year = {1981},
    publisher = {Clarendon Press},
    keywords = {physics}
}

@online{knuthwebsite,
    author = "Donald Knuth",
    title = "Knuth: Computers and Typesetting",
    url  = "http://www-cs-faculty.stanford.edu/~uno/abcde.html",
    addendum = "(accessed: 01.09.2016)",
    keywords = "latex,knuth"
}

@inbook{knuth-fa,
    author = "Donald E. Knuth",
    title = "Fundamental Algorithms",
    publisher = "Addison-Wesley",
    year = "1973",
    chapter = "1.2",
    keywords  = "knuth,programming"
}
```

--------------------------------

### Correct \verb command usage with delimiters

Source: https://www.overleaf.com/learn/latex/Errors/LaTeX_Error%3A_%5Cverb_ended_by_end_of_line

The \verb command requires matching start and end delimiters. Any character can be used as a delimiter, provided it does not appear within the text to be typeset verbatim. This example uses '!' as the delimiter.

```latex
\verb!\frac{1}{2}!
```

--------------------------------

### LaTeX Error Message Example

Source: https://www.overleaf.com/learn/latex/Errors/LaTeX_Error%3A_Missing_%5Cbegindocument

This is an example of the error message generated when \begin{document} is missing. Ensure \begin{document} is included after the preamble to resolve this.

```latex
main.tex, line 5
LaTeX Error: Missing \begin{document}.
See the LaTeX manual or LaTeX Companion for explanation. Type H <return> for immediate help. ... You're in trouble here. Try typing <return> to proceed. If that doesn't work, type X <return> to quit.
```

--------------------------------

### Basic TeX Document Example

Source: https://www.overleaf.com/learn/latex/Articles/The_TeX_family_tree%3A_LaTeX%2C_pdfTeX%2C_XeTeX%2C_LuaTeX_and_ConTeXt

A simple TeX document demonstrating basic typesetting and math rendering. Use \bye to end the document.

```tex
\TeX{} is good at typesetting words like `fjord', `efficiency',
and `fiasco'. It is also good at typesetting math like,
$a^2 + b^2 = c^2$.
\bye

```

--------------------------------

### Example Usage of `\codestoemoji` Macro

Source: https://www.overleaf.com/learn/latex/Articles/An_overview_of_technologies_supporting_the_use_of_colour_emoji_fonts_in_LaTeX

This is an example of how to use the `\codestoemoji` macro with a specific sequence of Unicode character codes representing an emoji.

```latex
\codestoemoji{\Uchar"1F3F4\Uchar"E0067\Uchar"E0062\Uchar"E0065\Uchar"E006E\Uchar"E0067\Uchar"E007F}


```

--------------------------------

### Example Using Named Colors with the color Package

Source: https://www.overleaf.com/learn/latex/Using_colours_in_LaTeX

Applies named colors using the 'color' package with 'usenames' and 'dvipsnames' options, producing similar output to the xcolor example for list items, rules, and text backgrounds.

```latex
\documentclass{article}
\usepackage[usenames,dvipsnames]{color} %using the color package, not xcolor
\begin{document}
This example shows how to use the \texttt{\bfseries color} package 
to change the color of \LaTeX{} page elements.

\begin{itemize}
\color{ForestGreen}
\item First item
\item Second item
\end{itemize}

\noindent
{\color{RubineRed} \rule{\linewidth}{0.5mm}}

The background color of text can also be \textcolor{red}{easily} set. For 
instance, you can change use an \colorbox{BurntOrange}{orange background} and then continue typing.
\end{document}
```

--------------------------------

### Basic French Document Setup with pdfLaTeX

Source: https://www.overleaf.com/learn/latex/French

Use this setup for documents requiring French language support, including correct hyphenation and accented characters. Ensure `babel` and `fontenc` are included for proper rendering. The `numprint` package is used for number formatting.

```latex
\documentclass{article}
% \usepackage[utf8]{inputenc} is no longer required (since 2018)

%Set the font (output) encoding
%--------------------------------------
\usepackage[T1]{fontenc} %Not needed by LuaLaTeX or XeLaTeX

%French-specific commands
%--------------------------------------
\usepackage[french]{babel}
\usepackage[autolanguage]{numprint} % for the \nombre command

%Hyphenation rules
%--------------------------------------
\usepackage{hyphenat}
\hyphenation{mate-mática recu-perar}
%--------------------------------------

\begin{document}
\tableofcontents

\vspace{2cm} %Add a 2cm space

\begin{abstract}
Ceci est un bref résumé du contenu du document écrit en français.
\end{abstract}

\section{Section d'introduction}
Il s'agit de la première section, nous ajoutons des éléments supplémentaires et tout sera correctement orthographiés. En outre, si un mot est trop long et doit être tronqué, babel va essayer de tronquer correctement en fonction de la langue.

\section{Section théorèmes}
Cette section est de voir ce qui se passe avec les commandes de texte qui définissent.

\begin{itemize}
\item premier élément
\item deuxième élément
\end{itemize}

\[ \lim x =  \theta + \nombre{152383.52} \\]
\end{document}
```

--------------------------------

### Mix Arabic (RTL) and LTR languages with LuaLaTeX

Source: https://www.overleaf.com/learn/latex/Multilingual_typesetting_on_Overleaf_using_babel_and_fontspec

Use babel with the bidi=basic option for LuaLaTeX, which offers comprehensive support for complex scripts and bidirectional typesetting. This example includes Arabic text and basic font setup.

```latex
% Arabic text in this example is from 
% https://github.com/latex3/babel/blob/main/samples/lua-arabic.tex
\documentclass{article}
\usepackage[english,bidi=basic]{babel}
\babelprovide[import,main]{arabic}
\babelfont{rm}{FreeSerif}
\babelfont{sf}{FreeSans}
\babelfont{tt}{FreeMono}
\begin{document}
الكهرمان اسمه باليونانية الإيلقطرون[3] (معرب ἤλεκτρον إيلكترون أي ذو
البريق، ومنه الإلكترون عند الفيزيائيين، وعليه تسمية الكهرباء في
الفارسية برق)، واشتق منه اسم فاعليتيه فسمي إلكترسمس (ηλεκτρ‌ισμός)
للدلالة على الكهرباء. أما باللاتينية فالكلمة للكهرباء هي إيلكترستاس
(ēlectricitās)، وهي مشتقة من إيلكتركس (ēlectricus) أي شبيه الكهرمان.
\end{document}

```

--------------------------------

### Example of numbered theorems, corollaries, and lemmas in LaTeX

Source: https://www.overleaf.com/learn/latex/Theorems_and_proofs

Demonstrates defining and using theorem-like environments with custom numbering schemes. Includes theorems numbered by section, corollaries reset by theorems, and lemmas sharing the theorem counter.

```latex
\documentclass{article}
\usepackage[english]{babel}
\newtheorem{theorem}{Theorem}[section]
\newtheorem{corollary}{Corollary}[theorem]
\newtheorem{lemma}[theorem]{Lemma}

\begin{document}
\section{Introduction}
Theorems can easily be defined:

\begin{theorem}
Let \(f\) be a function whose derivative exists in every point, then \(f\) is 
a continuous function.
\end{theorem}

\begin{theorem}[Pythagorean theorem]
\label{pythagorean}
This is a theorem about right triangles and can be summarised in the next 
equation 
\[ x^2 + y^2 = z^2 \]
\end{theorem}

And a consequence of theorem \ref{pythagorean} is the statement in the next 
corollary.

\begin{corollary}
There's no right rectangle whose sides measure 3cm, 4cm, and 6cm.
\end{corollary}

You can reference theorems such as \ref{pythagorean} when a label is assigned.

\begin{lemma}
Given two line segments whose lengths are \(a\) and \(b\) respectively there is a 
real number \(r\) such that \(b=ra\).
\end{lemma}

```

--------------------------------

### Default Display Style Math - TeXBook Example

Source: https://www.overleaf.com/learn/latex/Display_style_in_math_mode

This example shows a complex fraction typeset in the default display style. It is useful for observing the default behavior before applying style overrides.

```latex
\[
a_0+{1\over a_1+
      {1\over a_2+
        {1 \over a_3 + 
           {1 \over a_4}}}}
\]

```

--------------------------------

### LuaTeX INI file example (lualatex.ini)

Source: https://www.overleaf.com/learn/latex/Articles/The_two_modes_of_TeX_engines%3A_INI_mode_and_production_mode

This .ini file configures LuaTeX for format generation. It includes initialization steps and inputs core LaTeX files.

```tex
% tex-ini-files 2016-04-15: lualatex.ini
% Originally written 2008 by Karl Berry. Public domain.

\input luatexconfig.tex

\begingroup
  \catcode`\{=1 
  \catcode`\}=2 
 
  % Set up job name quoting before latex.ltx
  % Web2c pdfTeX/XeTeX quote job names containing spaces, but LuaTeX does
  % not do this at the engine level. The behaviour can be changed using
  % a callback. Originally this code was loaded via lualatexquotejobname.tex
  % but that required a hack around latex.ltx: the behaviour has been altered
  % to allow the callback route to be used directly.
  \global\everyjob{\directlua{require("lualatexquotejobname.lua")}} 
\endgroup

\input latex.ltx


```

--------------------------------

### Begin Definition for 'boxed' Environment

Source: https://www.overleaf.com/learn/latex/Environments%23Defining_a_new_environment

Shows the LaTeX code executed when the 'boxed' environment begins, which starts a center and tabular environment.

```latex
\begin{center}
    \begin{tabular}{|p{0.9\textwidth}|}
    \hline\\

```

--------------------------------

### Define Custom Environment with Nested Counters

Source: https://www.overleaf.com/learn/latex/Counters

Use \counterwithin to link a custom counter to a section counter, ensuring it resets with each new section. The starred version \counterwithin* prevents redefinition of \theexample. This setup is useful for creating numbered examples within sections.

```latex
\documentclass{article}
\counterwithin{equation}{section}
\newcounter{example}
\counterwithin*{example}{section}
\newenvironment{example}[1][]{\stepcounter{example}%\par\vspace{5pt}\noindent
\fbox{\textbf{Example~\thesection.\theexample}}%\hrulefill\par\vspace{10pt}\noindent\rmfamily}{\par\noindent\hrulefill\vrule width10pt height2pt depth2pt\par}
\begin{document}
\section{First equation}
Some introductory text...
\begin{example}
\begin{equation}
    f(x)=\frac{x}{1+x^2}
\end{equation}
\end{example}

\subsection{More detail}
\begin{example}
Here we discuss
\begin{equation}
    f(x)=\frac{x+1}{x-1}
\end{equation}
... but don't say anything
\end{example}

\subsubsection{Even more detail}
\begin{example}
\begin{equation}
    f(x)=\frac{x^2}{1+x^3}
\end{equation}
\begin{equation}
    f(x+\delta x)=\frac{(x+\delta x)^2}{1+(x+\delta x)^3}
\end{equation}
\end{example}

\section{Third equation}
\begin{example}
The following function...
\begin{equation}
    f_1(x)=\frac{x+1}{x-1}
\end{equation}
..is a function
\begin{equation}
    f_2(x)=\frac{x^2}{1+x^3}
\end{equation}
\begin{equation}
    f_3(x)=\frac{3+ x^2}{1-x^3}
\end{equation}
\end{example}
\end{document}
```

--------------------------------

### Spanish Text Example with pdfLaTeX

Source: https://www.overleaf.com/learn/latex/Spanish

Use this example to typeset Spanish text with correct hyphenation and accented characters. Ensure `fontenc` and `babel` with the 'spanish' option are included in the preamble.

```latex
\documentclass{article}

% Set the font (output) encodings
\usepackage[T1]{fontenc}

% \usepackage[utf8]{inputenc} is no longer required (since 2018)

% Spanish-specific commands
\usepackage[spanish]{babel}
\begin{document}
\tableofcontents

\vspace{2cm} %Add a 2cm space

\begin{abstract}
Este es un breve resumen del contenido del 
documento escrito en español.
\end{abstract}

\section{Sección introductoria}
Esta es la primera sección, podemos agregar 
algunos elementos adicionales y todo será 
escrito correctamente. Más aún, si una palabra 
es demasiado larga y tiene que ser truncada, 
babel tratará de truncarla correctamente 
dependiendo del idioma.

\section{Sección con teoremas}
Esta sección es para ver qué pasa con los comandos 
que definen texto
\end{document}
```

--------------------------------

### Complex Feynman Diagram Example 1

Source: https://www.overleaf.com/learn/latex/Feynman_diagrams

This example demonstrates creating a more complex Feynman diagram with labeled internal lines and a vertex. It utilizes the fmfgraph* environment and various fmf commands for drawing.

```latex
\documentclass{article}
\usepackage{feynmp-auto}
\begin{document}
\begin{fmffile}{complex-a}
\begin{fmfgraph*}(100,100)
    \fmfleft{i1}
    \fmfright{o1,o2}
    \fmf{fermion,label=$u$}{i1,w1}
    \fmf{fermion,label=$d$}{w1,o1}
    \fmf{photon,label=$W^{+}$}{w1,o2}
    \fmfv{lab=$V^{\ast}_{ud}$,lab.dist=0.05w}{w1}
\end{fmfgraph*}
\end{fmffile}
\end{document}
```

--------------------------------

### Coproduct with Limits (Reference)

Source: https://www.overleaf.com/learn/latex/Subscripts_and_superscripts

Reference example for a coproduct operator with specified lower and upper bounds.

```latex
`\coprod_{i=1}^n`
```

--------------------------------

### Basic Markdown Syntax Example

Source: https://www.overleaf.com/learn/latex/Articles/How_to_write_in_Markdown_on_Overleaf

Demonstrates fundamental Markdown syntax for headings, emphasis, links, and lists. This is how Markdown text is typically written before conversion.

```markdown
# Grocery list
*Remember* to grab as much as we can during upcoming [sales](http://acme-marg.com)!
## Food
- baked beans
- spaghetti
## Stationery 
- writing pad
- pencils

```

--------------------------------

### Basic CD Calendar Setup

Source: https://www.overleaf.com/learn/latex/Articles/How_to_create_a_multilingual%2C_customisable_CD_disk_jewel_case_calendar_using_LaTeX

Sets up a 12pt document for a CD jewel case calendar and includes two months. Use `\clearpage` to separate months.

```latex
\documentclass[12pt]{cdcalendar}
\begin{document}
    
%% June 2015
\monthCalendar{2015}{06}
\clearpage
    
%% July 2015
\monthCalendar{2015}{07}
\end{document}
```

--------------------------------

### Misspelled environment example

Source: https://www.overleaf.com/learn/latex/Errors/LaTeX_Error%3A_Environment_XXX_undefined

This example demonstrates a common spelling mistake where 'list' is incorrectly written as 'lisQ', leading to an 'Environment XXX undefined' error. Always check environment spelling.

```latex
\begin{lisQ}
    \item This is a list item
    \item This is another list item
\end{lisQ}


```

--------------------------------

### Remove Counter Link with `\counterwithout`

Source: https://www.overleaf.com/learn/latex/Counters%23Introduction_to_LaTeX_counters

Demonstrates using `\counterwithout{example}{section}` to make the `example` counter independent of the `section` counter. This prevents the `example` counter from resetting at the beginning of each new section.

```latex
\section{First equation}
\begin{example}
\begin{equation}
    f(x)=\frac{x}{1+x^2}
\end{equation}
\end{example}

\subsection{Second equation}
\begin{example}
\begin{equation}
    f(x)=\frac{x+1}{x-1}
\end{equation}
\end{example}
\vspace{6pt}\noindent Note how the \texttt{example} counter is reset at the start Section \ref{sec:n0}. 
\section{Third equation}
\label{sec:n0}
\begin{example}
\begin{equation}
    f(x)=\frac{x+1}{x-1}
\end{equation}
\end{example}
\vspace{6pt}\noindent Here, we wrote \verb|\counterwithout{example}{section}| so that \texttt{example} is no longer reset at the start of a section. In Sections \ref{sec:n1} and \ref{sec:n2} the \texttt{example} counter keeps increasing. \counterwithout{example}{section}
\section{Fourth equation}
\label{sec:n1}
\begin{example}
\begin{equation}
    f(x)=\frac{x^2+x^3}{1+x^3}
\end{equation}
\end{example}
\section{Fifth equation}
\label{sec:n2}
\begin{example}
\begin{equation}
    f(x,k)=\frac{x^2-x^k}{1+x^3}
\end{equation}
\end{example}
```

--------------------------------

### Enable SyncTeX: Output .synctex

Source: https://www.overleaf.com/learn/latex/MLTeX_EncTeX_and_SyncTeX_TeX_extensions

Use this command-line option to enable SyncTeX and generate an uncompressed synchronization file (.synctex).

```bash
pdftex -synctex=-1 myfile.tex
```

--------------------------------

### Enable SyncTeX: Output .synctex.gz

Source: https://www.overleaf.com/learn/latex/MLTeX_EncTeX_and_SyncTeX_TeX_extensions

Use this command-line option to enable SyncTeX and generate a gzipped synchronization file (.synctex.gz).

```bash
pdftex -synctex=1 myfile.tex
```

--------------------------------

### Example \hbox with Glue Settings

Source: https://www.overleaf.com/learn/latex/Articles/How_TeX_Calculates_Glue_Settings_in_an_%5Chbox

This code defines a horizontal box with characters and various types of horizontal glue, each with specified stretch and shrink values. It serves as a basis for calculating the box's natural width (WN) and understanding how excess space is distributed.

```tex
\hbox to100pt{
A\hskip4pt plus3pt minus 2pt% B\hskip 0pt plus 2fil% 
C\hskip 0pt plus 2fill%
D\hskip 0pt plus 3fill%
}

```

--------------------------------

### LaTeX: Basic Font Size and Style Example

Source: https://www.overleaf.com/learn/latex/Font_sizes%2C_families%2C_and_styles

Demonstrates using the \tiny command for the smallest font size and \textsc for small caps style. Text within braces is affected.

```latex
This is a simple example, {\tiny this will show different font sizes} and also \textsc{different font styles}.

```

--------------------------------

### Demonstrating \textstyle, \scriptstyle, and \scriptscriptstyle

Source: https://www.overleaf.com/learn/latex/Display_style_in_math_mode

This example illustrates the effects of \textstyle, \scriptstyle, and \scriptscriptstyle on a summation formula, showing how each command alters the size and positioning of elements for different contexts like subscripts and second-order subscripts.

```latex
\[
\begin{align*}
f(x) = \sum_{i=0}^{n} \frac{a_i}{1+x} \\
\textstyle f(x) = \textstyle \sum_{i=0}^{n} \frac{a_i}{1+x} \\
\scriptstyle f(x) = \scriptstyle \sum_{i=0}^{n} \frac{a_i}{1+x} \\
\scriptscriptstyle f(x) = \scriptscriptstyle \sum_{i=0}^{n} \frac{a_i}{1+x}
\end{align*}
\]

```

--------------------------------

### Basic Mathematical Font Example

Source: https://www.overleaf.com/learn/latex/Mathematical_fonts

Demonstrates the use of calligraphic font for representing a topological space and its basis.

```latex
Let \( \mathcal{T} \) be a topological space, a basis is defined as
\[
 \mathcal{B} = \{B_{\alpha} \in \mathcal{T}\, |\,  U = \bigcup B_{\alpha} \forall U \in \mathcal{T} \}
\]
```

--------------------------------

### Typeset output of \jobname example

Source: https://www.overleaf.com/learn/latex/Articles/How_does_%5Cexpandafter_work%3A_TeX_uses_temporary_token_lists

Shows the rendered output when the \jobname primitive is expanded in the preceding TeX code.

```text
    The name of my file is mycode.tex
```

--------------------------------

### LaTeX: Flexible \baselineskip example

Source: https://www.overleaf.com/learn/latex/Articles/How_to_change_paragraph_spacing_in_LaTeX

Illustrates setting \baselineskip with a flexible 'stretch' component using 'plus 1fil'. This allows white space between lines to stretch as needed to fill the page.

```latex
\baselineskip=12pt plus1fil

```

```latex
\documentclass{article}
\usepackage{blindtext}
\title{Flexible \texttt{\string\baselineskip} demo}
\author{Overleaf}
\date{January 2022}

\begin{document}

\maketitle
We’ll provide \verb|\baselineskip| with a very flexible ``stretch'' component by assigning the value \texttt{12pt plus1fil}: \verb|\baselineskip=12pt plus1fil|.\baselineskip=12pt plus1fil 
\section{Introduction}
Add a paragraph that will now stretch to fill the page. \blindtext

\end{document}


```

--------------------------------

### Configure Page Layout with Geometry Package

Source: https://www.overleaf.com/learn/latex/Creating_a_document_in_LaTeX

The geometry package allows detailed configuration of page layout. This example sets the total page size and margins.

```latex
\usepackage[total={6.5in,8.75in},
top=1.2in, left=0.9in, includefoot]{geometry}
```

--------------------------------

### Summation with Limits (Reference)

Source: https://www.overleaf.com/learn/latex/Subscripts_and_superscripts

Reference example for a summation operator with specified lower and upper bounds.

```latex
`\sum_{i=1}^{\infty}`
```

--------------------------------

### Example: Printing Various Counter Formats

Source: https://www.overleaf.com/learn/latex/Counters%23Introduction_to_LaTeX_counters

Demonstrates the usage of \arabic, \roman, \Roman, \alph, \Alph, and \fnsymbol commands to display a counter's value in different formats.

```latex
\newcounter{somecounter}
\setcounter{somecounter}{9}
\begin{itemize}
    \item \verb|\arabic{somecounter}| typesets the \texttt{somecounter} value of  \thesomecounter{} as \arabic{somecounter} 
    \item \verb|\roman{somecounter}| typesets the \texttt{somecounter} value of  \thesomecounter{} as  \roman{somecounter}
    \item \verb|\Roman{somecounter}| typesets the \texttt{somecounter} value of  \thesomecounter{} as  \Roman{somecounter}
    \item \verb|\alph{somecounter}| typesets the \texttt{somecounter} value of  \thesomecounter{} as  \alph{somecounter}
   \item \verb|\Alph{somecounter}| typesets the \texttt{somecounter} value of  \thesomecounter{} as  \Alph{somecounter}
    \item \verb|\fnsymbol{somecounter}| typesets the \texttt{somecounter} value of  \thesomecounter{} as  \fnsymbol{somecounter}
\end{itemize}

```

--------------------------------

### Complex Feynman Diagram Example 3 (Gluon Interaction)

Source: https://www.overleaf.com/learn/latex/Feynman_diagrams

This example illustrates a complex Feynman diagram involving gluon interactions and quark loops. It uses \fmfblob for visual elements and defines custom line styles using \fmfi.

```latex
\documentclass{article}
\usepackage{feynmp-auto}
\begin{document}
\begin{fmffile}{complex-c}
\begin{fmfgraph*}(200,200)
    %bottom and top verticies
    \fmfbottom{P1,P2}
    \fmftop{P1',b,bbar,P2'}
    %incoming protons to gluon vertices
    \fmf{fermion,tension=2,lab=$P_1$}{P1,g1}
    \fmf{fermion,tension=2,lab=$P_2$}{P2,g2}
    %blobs at gluon vertices, 0.16w is the size of blob
    \fmfblob{.16w}{g1,g2}
    %gluon from P1 to vertex1
    \fmf{gluon,lab.side=right,lab=$x_{1}P_{1}$}{g1,v1}
    %gluon from P2 to vertex2 - note change of order!
    \fmf{gluon,lab.side=right,lab=$x_{2}P_{2}$}{v2,g2}
    %quark loop was here
    \fmf{fermion, tension=.6, lab.side=right,lab=$b$}{v1,b}
    \fmf{fermion, tension=1.2}{v2,v1}
    \fmf{fermion, tension=.6, lab.side=right,lab=$\\overline{b}$}{bbar,v2}
    %outgoing protons
    \fmf{fermion}{g1,P1'}
    \fmf{fermion}{g2,P2'}
    %freeze everything in place
    \fmffreeze
    \renewcommand{\\P}[3]{\\fmfi{plain}{% 
        vpath(__#1,__#2) shifted (thick*(#3))}}
    %lines on P1
    \P{P1}{g1}{2,0}
    \P{P1}{g1}{-2,1}
    %lines on p2
    \P{P2}{g2}{2,1}
    \P{P2}{g2}{-2,0}
    %lines on P1'
    \P{g1}{P1'}{-2,-1}
    \P{g1}{P1'}{2,0}
    %lines on P2'
    \P{g2}{P2'}{-2,0}
    \P{g2}{P2'}{2,-1}
\end{fmfgraph*}
\end{fmffile}
\end{document}
```

--------------------------------

### Underscore in Filename (Error Example)

Source: https://www.overleaf.com/learn/latex/Errors/Missing_%24_inserted

This example demonstrates how an underscore in a filename outside of math mode triggers the 'Missing $ inserted' error.

```latex
Using a math character, such as an underscore, in a file name: math_example.tex.
```

--------------------------------

### LaTeX Table with booktabs Package

Source: https://www.overleaf.com/learn/how-to/How_to_insert_tables_in_Overleaf

This example demonstrates how to create a table using the `booktabs` package for enhanced horizontal rules. It includes commands like \toprule, \midrule, and \bottomrule for professional table formatting.

```latex
\documentclass{article}
\usepackage{booktabs}
\usepackage{hologo} % for the XeTeX logo
\begin{document}
\begin{table}
\begin{tabular}{lcc}
\toprule 
    \TeX{} engine&Year released&Native UTF-8\\
\midrule 
    pdf\TeX&1996&No\ 
    \hologo{XeTeX}&2004&Yes\ 
    Lua\TeX&2007&Yes\ 
    LuaHB\TeX&2019&Yes\ 
\bottomrule
\end{tabular}
\end{table}
\end{document}
```

--------------------------------

### Start New Paragraph with \par Command

Source: https://www.overleaf.com/learn/latex/Paragraphs_and_new_lines

Insert the \texttt{\\par} command to explicitly start a new paragraph. This is an alternative to using a blank line between text blocks.

```latex
This is text contained in the first paragraph. 
This is text contained in the first paragraph. 
This is text contained in the first paragraph.\par
This is text contained in the second paragraph. 
This is text contained in the second paragraph.
This is text contained in the second paragraph.


```

--------------------------------

### Forbidden Command in Math Mode (Error Example)

Source: https://www.overleaf.com/learn/latex/Errors/Missing_%24_inserted

This example shows the 'Missing $ inserted' error when a TeX primitive command like `\vskip` is used within mathematical mode.

```latex
\documentclass{article}
\begin{document}
I want to add some space, but this is not the way to do it...
\[y=f(x) \vskip5pt z=f(y)\]
$y=f(x) \vskip5pt z=f(y)$
\end{document}
```

--------------------------------

### Visualize LaTeX Layout with the layout Package

Source: https://www.overleaf.com/learn/latex/Page_size_and_margins

Demonstrates how to use the \layout command from the layout package to visualize the current page dimensions and parameters. This is useful for debugging and understanding page setup.

```latex
\documentclass{article}
\usepackage{layout}
\begin{document}
\section{Default \LaTeX{} layout}
Here's the default layout:

\vspace{10pt}
\layout
\section{Make some changes}
Make changes to the margin paragraph settings and use the command \verb|layout*| to redraw the page layout diagram:
\vspace{10pt}
\setlength{\marginparwidth}{0pt}
\setlength{\marginparsep}{0pt}

\layout*
\end{document}
```

--------------------------------

### XeLaTeX Example with TwemojiMozilla.ttf (Fails)

Source: https://www.overleaf.com/learn/latex/Articles/An_overview_of_technologies_supporting_the_use_of_colour_emoji_fonts_in_LaTeX

This XeLaTeX example attempts to use the TwemojiMozilla.ttf font but fails to render color emoji in the PDF. XeTeX generates an intermediate .xdv file which `xdvipdfmx` cannot convert with color glyph data.

```latex
\documentclass{article}
\usepackage{fontspec}
\begin{document}
\newfontfamily\emojifont{TwemojiMozilla.ttf}
\newcommand{\smiley}{{\emojifont\char"1F600}}
Here is a smiley: \smiley
\end{document}

```

--------------------------------

### Create a Simple Glossary in LaTeX

Source: https://www.overleaf.com/learn/latex/Glossaries

Import the glossaries package and use \newglossaryentry to define terms. Use \gls to reference terms and \printglossaries to display the glossary.

```latex
\documentclass{article}
\usepackage[utf8]{inputenc}
\usepackage{glossaries}

\makeglossaries

\newglossaryentry{latex}
{
    name=latex,
    description={Is a markup language specially suited 
    for scientific documents}
}

\newglossaryentry{maths}
{
    name=mathematics,
    description={Mathematics is what mathematicians do}
}

\title{How to create a glossary}
\author{ }
\date{ }

\begin{document}
\maketitle

The \Gls{latex} typesetting markup language is specially suitable 
for documents that include \gls{maths}. 

\clearpage

\printglossaries

\end{document>
```

--------------------------------

### Contour Integral with Limits (Reference)

Source: https://www.overleaf.com/learn/latex/Subscripts_and_superscripts

Reference example for a contour integral operator with specified lower and upper bounds.

```latex
`\oint_{i=1}^n`
```

--------------------------------

### Basic CircuiTikz Diagram with Variable Inductor

Source: https://www.overleaf.com/learn/latex/CircuiTikz_package

This example demonstrates the basic usage of the `circuitikz` environment and a 'variable cute inductor' node with standard TikZ syntax.

```latex
\documentclass{article}
\usepackage{circuitikz}
\begin{document}
\begin{center}
\begin{circuitikz}
\draw (0,0) to[ variable cute inductor ] (2,0); 
\end{circuitikz}
\end{center}
\end{document}
```

--------------------------------

### Inline Math Mode Examples in LaTeX

Source: https://www.overleaf.com/learn/latex/Mathematical_expressions

Illustrates three common ways to typeset inline mathematical expressions in LaTeX: using \(...\), $...$, and \begin{math}...\end{math}.

```latex
\documentclass{article}
\begin{document}

\noindent Standard \LaTeX{} practice is to write inline math by enclosing it between \verb|\(...\)|:

\begin{quote}
In physics, the mass-energy equivalence is stated 
by the equation \(E=mc^2\), discovered in 1905 by Albert Einstein.
\end{quote}

\noindent Instead if writing (enclosing) inline math between \verb|\(...\)| you can use \texttt{\$...\$} to achieve the same result:

\begin{quote}
In physics, the mass-energy equivalence is stated 
by the equation $E=mc^2$, discovered in 1905 by Albert Einstein.
\end{quote}

\noindent Or, you can use \verb|\begin{math}...\end{math}|:

\begin{quote}
In physics, the mass-energy equivalence is stated 
by the equation \begin{math}E=mc^2\end{math}, discovered in 1905 by Albert Einstein.
\end{quote}
\end{document}

```

--------------------------------

### Union with Limits (Reference)

Source: https://www.overleaf.com/learn/latex/Subscripts_and_superscripts

Reference example for a union operator with specified lower and upper bounds.

```latex
`\cup_{i=1}^n`
```

--------------------------------

### Basic Italian Document Setup with pdfLaTeX

Source: https://www.overleaf.com/learn/latex/Italian

This snippet demonstrates the essential LaTeX packages and commands for typesetting Italian text. It includes font encoding, language support via `babel`, and custom hyphenation rules. Use this for standard Italian documents.

```latex
\documentclass{article}

%\usepackage[utf8]{inputenc} is no longer required (since 2018)

%Set the font (output) encoding
%--------------------------------------
\usepackage[T1]{fontenc} %Not needed by LuaLaTeX or XeLaTeX
%--------------------------------------

%Italian-specific commands
%--------------------------------------
\usepackage[italian]{babel}
%Hyphenation rules
%--------------------------------------
\usepackage{hyphenat}
\hyphenation{mate-mati-ca recu-perare}

\begin{document}
\tableofcontents

\vspace{2cm} %Add a 2cm space

\begin{abstract}
Questo è un breve riassunto dei contenuti del 
documento scritto in italiano.
\end{abstract}

\section{Sezione introduttiva}
Questa è la prima sezione, possiamo aggiungere 
alcuni elementi aggiuntivi e tutto 
digitato correttamente. Inoltre, se una parola 
è troppo lunga e deve essere troncato 
babel cercherà per troncare correttamente 
a seconda della lingua.

\section{Teoremi Sezione}
Questa sezione è quello di vedere cosa succede con i comandi 
testo definendo

\[ \lim x =  \sin{\theta} + \max \{3.52, 4.22\} \]
\end{document}

```

--------------------------------

### Comparing `singlespace` and `singlespace*` environments

Source: https://www.overleaf.com/learn/latex/Articles/How_to_change_paragraph_spacing_in_LaTeX

This example illustrates the difference in vertical spacing around itemized lists produced by the `singlespace` and `singlespace*` environments. The `singlespace*` environment is noted to provide improved vertical spacing around list environments.

```latex
\doublespacing 
\blindtext[1]

\begin{minipage}{.4\textwidth}
\begin{singlespace}
Here is a bulleted list\par (\texttt{singlespace} environment).
\begin{itemize}
\item One
\item Two
\item Three
\end{itemize}
\end{singlespace}%
The list has ended.
\end{minipage}\kern10pt%
\begin{minipage}{.4\textwidth}
\begin{singlespace*}
Here is a bulleted list\par (\texttt{singlespace*} environment).
\begin{itemize}
\item One
\item Two
\item Three
\end{itemize}
\end{singlespace*}
The list has ended.
\end{minipage}
```

--------------------------------

### Demonstrating \prevdepth with \hbox

Source: https://www.overleaf.com/learn/latex/Articles/How_to_change_paragraph_spacing_in_LaTeX

This example demonstrates how setting \prevdepth to -1000pt immediately after an \hbox prevents interline glue between that box and the next. It highlights that \prevdepth is updated after each box is processed.

```latex
\noindent The following \verb|\hbox|es are processed when \TeX{} is in so-called \textit{outer vertical mode}. After processing (typesetting), these boxes are appended to the current page and stacked vertically with interline glue inserted between them:

\hbox{j}
\hbox{p}
\hbox{g}

Now, immediately after after the box \verb|\hbox{j}|, we set \verb|\prevdepth| to \texttt{-1000pt} which prevents interline glue being placed between the boxes \verb|\hbox{j}| and \verb|\hbox{p}|: 

\hbox{j}\setlength{\prevdepth}{-1000pt}
\hbox{p}
\hbox{g}

Observe that interline glue \textit{is} placed between the boxes \verb|\hbox{p}| and \verb|\hbox{g}| because the value of \verb|\prevdepth| is updated after \verb|\hbox{p}| is processed and added to the list: the value of \verb|\prevdepth| becomes the depth of the box \verb|\hbox{p}|. When \verb|\hbox{g}| is added to the vertical list the value of \verb|\prevdepth| is no longer \texttt{-1000pt}; consequently, interline glue can now be added.


```

--------------------------------

### Applying custom spacing with the `spacing` environment

Source: https://www.overleaf.com/learn/latex/Articles/How_to_change_paragraph_spacing_in_LaTeX

This example demonstrates using the `spacing` environment with a `\baselinestretch` value of 2.5 to apply custom spacing to a section of text. It also shows how to revert to single spacing.

```latex
\documentclass{article}
\usepackage[paperheight=18cm,paperwidth=14cm,textwidth=12cm]{geometry}
\usepackage{blindtext}
\usepackage{setspace}
\doublespacing
\begin{document}
\blindtext[1]

Now, apply a larger (custom) spacing by applying a \verb|\baselinestretch| value of 3:

\begin{verbatim}
\begin{spacing}{3}
\blindtext[1]
\end{spacing}
\end{verbatim}

\begin{spacing}{2.5}
\blindtext[1]
\end{spacing}
Now, revert to \verb|\singlespacing|\singlespacing\blindtext[1]
\end{document}
```

--------------------------------

### Create a Description List with description

Source: https://www.overleaf.com/learn/latex/Lists

The `description` environment creates lists where each item can have a custom label specified in square brackets after the `\item` command.

```latex
\documentclass{article}
\usepackage[english]{babel} % To obtain English text with the blindtext package
\usepackage{blindtext}
\begin{document}

\begin{description}
   \item This is an entry \textit{without} a label.
   \item[Something short] A short one-line description.
   \item[Something long] A much longer description. \blindtext[1]
\end{description}
\end{document}

```

--------------------------------

### LuaLaTeX Example with SVG Font (using luaotfload)

Source: https://www.overleaf.com/learn/latex/Articles/An_overview_of_technologies_supporting_the_use_of_colour_emoji_fonts_in_LaTeX

This example demonstrates loading an SVG-based OpenType color font directly using `luaotfload`. It is recommended to omit the `mode=harf` option for this method to function correctly. Replace `your SVG font file name here` with the actual font file name.

```latex
\font\emoji=[your SVG font file name here]:+svg;

```

--------------------------------

### Example: Modifying \parindent and \parskip

Source: https://www.overleaf.com/learn/latex/Articles/How_to_change_paragraph_spacing_in_LaTeX

This example demonstrates how to set and modify \parindent and \parskip to control paragraph indentation and spacing between paragraphs. It shows the effect of different values, including negative \parindent.

```latex
\documentclass{article}
% Choose a conveniently small page size
\usepackage[paperheight=16cm,paperwidth=12cm,textwidth=8cm]{geometry}
% Create a command to hold a paragraph of text
\newcommand{\testpar}{\texttt{\string\parindent=\the\parindent\ }When \TeX{} typesets a paragraph it treats each individual character as a 2D ``box'' with a specific width, height and depth.\par}
\begin{document}

% Set \parskip to put 10pt between paragraphs
\setlength{\parskip}{10pt}

% Set the value of \parindent to 0pt
\setlength{\parindent}{0pt}
\testpar

% Set the value of \parindent to 10pt
\setlength{\parindent}{10pt}
\testpar

% Set \parindent in a group
{\setlength{\parindent}{50pt}\testpar}

% Now \parindent is again 10pt
\testpar

% Yes, you can have a negative \parindent
\setlength{\parindent}{-20pt}
\testpar
\end{document}
```

--------------------------------

### TeX Error Message Example

Source: https://www.overleaf.com/learn/latex/Articles/What_is_a_TeX_token_list

An example of a typical TeX error message that can occur when macro delimiters do not match due to category code changes.

```tex
Runaway argument?
THIS TEXT defz 
! Paragraph ended before \mymacro was complete.
<to be read again>
\par
l.22

```

--------------------------------

### Minimal parskip package example

Source: https://www.overleaf.com/learn/latex/Articles/How_to_change_paragraph_spacing_in_LaTeX

Loads the `parskip` package without options to demonstrate default paragraph spacing. It also shows the values of `\baselineskip`, `\parskip`, and `\parindent`.

```latex
\documentclass{article}
% Choose a conveniently small page size
\usepackage[paperheight=16cm,paperwidth=14cm,textwidth=12cm]{geometry}
% Load blindtext package for dummy text
\usepackage{blindtext}
% Load the parskip package without options
\usepackage{parskip}
\begin{document}

\blindtext[1]\par %Use \par to force a new paragraph
\blindtext[1]

The value of \verb|\parindent| is \texttt{\the\parindent}. Here are the other values:

\begin{itemize}
\item The value of \verb|\baselineskip| is \texttt{\the\baselineskip}
\item The value of \verb|\parskip| is \texttt{\the\parskip}
\end{itemize}
\end{document}
```

--------------------------------

### Intersection with Limits (Reference)

Source: https://www.overleaf.com/learn/latex/Subscripts_and_superscripts

Reference example for an intersection operator with specified lower and upper bounds.

```latex
`\cap_{i=1}^n`
```

--------------------------------

### Example Usage of Scoop Macros

Source: https://www.overleaf.com/learn/latex/Articles/TeX_Tables%3A_How_TeX_Calculates_Spanned_Column_Widths

Demonstrates how to use the \beginscoop and \endscoop macros to enclose TeX table code, such as an \halign command, for later processing.

```tex
\beginscoop
\halign{...
}\endscoop

```

--------------------------------

### Manual Vertex Placement in Feynman Diagrams

Source: https://www.overleaf.com/learn/latex/Feynman_diagrams

Demonstrates manual placement of vertices using the \vertex command within a \begin{feynman} environment. This method provides precise control over vertex positions, especially for complex diagrams.

```latex
\begin{tikzpicture}
  \begin{feynman}
    \vertex (a) {\(\mu^{-}\)};
    \vertex [right=of a] (b);
    \vertex [above right=of b] (f1) {\(\nu_{\mu}\)};
    \vertex [below right=of b] (c);
    \vertex [above right=of c] (f2) {\(\overline \nu_{e}\)};
    \vertex [below right=of c] (f3) {\(e^{-}\)};

    \diagram* {
      (a) -- [fermion] (b) -- [fermion] (f1),
      (b) -- [boson, edge label'=\(W^{-}\] (c),
      (c) -- [anti fermion] (f2),
      (c) -- [fermion] (f3),
    };
  \end{feynman}
\end{tikzpicture}

```

--------------------------------

### Complex Feynman Diagram Example 2

Source: https://www.overleaf.com/learn/latex/Feynman_diagrams

This advanced example showcases a more intricate Feynman diagram, including features like tension control for vertex positioning and labeling of vertices with complex notation. It uses \fmffreeze to fix elements.

```latex
\documentclass{article}
\usepackage{feynmp-auto}
\begin{document}
\begin{fmffile}{complex-b}
\begin{fmfgraph*}(200,200)
    % bottom and top verticies
    \fmfstraight
    \fmfleft{i0,i1,i2,id1,id2,i3,i4,i5}
    \fmfright{o0,o1,o2,od1,od2,o3,o4,o5}
    % incoming proton to gluon vertices
    \fmf{fermion,label=$d$}{i1,o1}
    % tension shifts vertex to one side
    \fmf{fermion,tension=1.5,label=$\\overline{b}$}{v2,i4}
    \fmf{fermion,label=$\\overline{c}$}{o4,v2}
    \fmffreeze
    \fmf{fermion}{o2,v3,o3}
    \fmf{fermion,label=$\\overline{s}$}{o2,v3}
    \fmf{fermion,label=$c$}{v3,o3}
    \fmf{photon, tension=2,label=$W^{+}$}{v2,v3}
    % phantom centres the W->cs vertex
    \fmf{phantom,tension=1.5}{i1,v3}

    \fmfv{lab=$V_{cb}^{\\ast}$}{v2}
    \fmfv{lab=$V_{cs}$,lab.dist=-.1w}{v3}
\end{fmfgraph*}
\end{fmffile}
\end{document}
```

--------------------------------

### LuaLaTeX Example with TwemojiMozilla.ttf (Works)

Source: https://www.overleaf.com/learn/latex/Articles/An_overview_of_technologies_supporting_the_use_of_colour_emoji_fonts_in_LaTeX

This LuaLaTeX example successfully renders color emoji using TwemojiMozilla.ttf by specifying `Renderer=HarfBuzz` in the `fontspec` package. This configuration enables the HarfBuzz shaping engine for proper font rendering.

```latex
\documentclass{article}
\usepackage{fontspec}
\begin{document}
\newfontfamily\emojifont{TwemojiMozilla.ttf}[Renderer=HarfBuzz]
\newcommand{\smiley}{{\emojifont\char"1F600}}
Here is a smiley: \smiley
\end{document}

```

--------------------------------

### TikZposter Preamble and Title Setup

Source: https://www.overleaf.com/learn/latex/Posters

Sets up the basic document class, title, author, date, and institute for a tikzposter. Includes theme selection and printing the title.

```latex
\documentclass[25pt, a0paper, portrait]{tikzposter}
\title{Tikz Poster Example}
\author{Overleaf Team}
\date{\today}
\institute{Overleaf Institute}
\usetheme{Board}

\begin{document}

\maketitle

\end{document}

```

--------------------------------

### Display hb-view Help

Source: https://www.overleaf.com/learn/latex/Articles/An_overview_of_technologies_supporting_the_use_of_colour_emoji_fonts_in_LaTeX

View all available command-line options for the `hb-view` utility to explore its full capabilities.

```bash
hb-view --help-all
```

--------------------------------

### LaTeX Index Style File Configuration

Source: https://www.overleaf.com/learn/latex/Indices

Configure the appearance of your LaTeX index using a style file. This example sets up heading formatting, item spacing, and delimiters.

```latex
headings_flag 1

heading_prefix "\centering\large\sffamily\bfseries%\n\noindent\textbf{"
heading_suffix "}\par\nopagebreak\n"

item_0 " \item \small "

delim_0 " \hfill "
delim_1 " \hfill "
delim_2 " \hfill "

```

--------------------------------

### Basic LaTeX Environment Structure

Source: https://www.overleaf.com/learn/latex/Environments%23Defining_a_new_environment

Illustrates the fundamental structure of a LaTeX environment using \begin and \end commands.

```latex
\begin{name}
Your content here...
...goes here...
\end{name}
```

--------------------------------

### Complete Arabic Document Setup with arabtex and babel

Source: https://www.overleaf.com/learn/latex/Arabic

This snippet shows a full LaTeX document setup for Arabic text using `arabtex` and `babel`. It includes necessary packages for input encoding, font encoding (implicitly handled by babel's arabic option), and language support. Use this for documents requiring Arabic content and standard LaTeX features like titles and tables of contents.

```latex
\documentclass[11pt,a4paper]{report}
\usepackage[utf8]{inputenc}
\usepackage{arabtex}
%\usepackage[LAE]{fontenc} %Not needed due to [arabic] option of the babel package
\usepackage[arabic]{babel}
\title{
    \Huge\textsc{اللغة العربية}
}
\author{سالم البوزيدي}
\begin{document}
\maketitle
\tableofcontents
\chapter{علوم الحاسوب}
\section{تاريخ}
\begin{otherlanguage}{arabic}
يعود تاريخ علوم الحاسوب إلى اختراع أول حاسوب رقمي حديث. فقبل العشرينات من القرن العشرين، كان مصطلح حاسوب \textLR{Computer} يشير إلى أي أداة بشرية تقوم بعملية الحسابات. ما هي القضايا أو الأشياء التي يمكن لآلة أن تحسبها باتباع قائمة من التعليمات مع ورقة وقلم، دون تحديد للزمن اللازم ودون أي مهارات أو بصيرة (ذكاء)؟ وكان أحد دوافع هذه الدراسات هو تطوير آلات حاسبة \textLR{computing machines} يمكنها إتمام الأعمال الروتينية والعرضة للخطأ البشري عند إجراء حسابات بشرية.
خلال الأربعينات، مع تطوير آلات حاسبة أكثر قوة وقدرة حسابية، تتطور مصطلح حاسوب ليشير إلى الآلات بدلا من الأشخاص الذين يقومون بالحسابات. وأصبح من الواضح أن الحواسيب يمكنها أن تقوم بأكثر من مجرد عمليات حسابية وبالتالي انتقلوا لدراسة تحسيب أو التحسيب بشكل عام. بدأت المعلوماتية وعلوم الحاسب تأخذ استقلالها كفرع أكاديمي مستقل في الستينات، مع إيجاد أوائل أقسام علوم الحاسب في الجامعات وبدأت الجامعات تعطي إجازات في هذه العلوم [1]. 
\end{otherlanguage}
\end{document}
```

--------------------------------

### Basic pLaTeX document example

Source: https://www.overleaf.com/learn/latex/Japanese%23The_pTeX_engine

A simple LaTeX document using the `jsarticle` class, intended for compilation with pLaTeX.

```latex
\documentclass{jsarticle}
\bibliographystyle{jplain}
\title{A pLaTeX example}
\begin{document}

本稿では、文書組版システムp\LaTeX{}の使い方を解説します。p\LaTeX{}を利用するときには、
あらかじめ文章中に\TeX{}コマンドと呼ばれる組版用の指示を混在させ\ldots

\section{導入}
こんにちは世界！
\end{document}
```

--------------------------------

### Compare \doublespacing with \begin{spacing}

Source: https://www.overleaf.com/learn/latex/Articles/How_to_change_paragraph_spacing_in_LaTeX

This LaTeX example compares the \baselinestretch effect of the \doublespacing command with a manual \begin{spacing} environment. It prints the resulting \baselineskip values for both methods, showing they are identical when using the calculated \baselinestretch of 1.618 for 11pt font.

```latex
\begingroup
\doublespacing
\verb|\baselineskip|=\texttt{\the\baselineskip} (via \verb|\doublespacing|)

\blindtext[1]

\endgroup

\begin{spacing}{1.618}
\verb|\baselineskip|=\texttt{\the\baselineskip} (via \verb|\begin{spacing}{1.618})|

\blindtext[1]
\end{spacing}
```

--------------------------------

### Basic LaTeX Document Structure with Sections

Source: https://www.overleaf.com/learn/latex/Sections_and_chapters

Demonstrates the basic usage of \section to create numbered sections in a LaTeX document. Includes document setup, title, author, and date.

```latex
\documentclass{article}
\usepackage{blindtext}

\title{Sections and Chapters}
\author{Overleaf}
\date{\today}

\begin{document}
\maketitle
\section{Introduction}

This is the first section.

\blindtext

\section{Second Section}
This is the second section

\blindtext
\end{document}
```

--------------------------------

### Basic Picture Environment with Default Origin

Source: https://www.overleaf.com/learn/latex/Picture_environment

Demonstrates the default behavior of the \begin{picture} environment where (0,0) is at the command's execution point. The \fbox command visualizes the bounding box.

```latex
\documentclass{article}
\usepackage[pdftex]{pict2e}
\usepackage[dvipsnames]{xcolor}
\begin{document}
\setlength{\unitlength}{1cm}
\setlength{\fboxsep}{0pt}

This is my picture\fbox{
\begin{picture}(3,3)
\put(0,0){{\color{blue}\circle*{0.25}}\hbox{\kern3pt \texttt{(0,0)}}}
\put(3,3){{\color{red}\circle*{0.25}}\hbox{\kern3pt \texttt{(3,3)}}}
\end{picture}}
\end{document}
```

--------------------------------

### Highlight Text and Use Block Environments

Source: https://www.overleaf.com/learn/latex/Beamer

Utilize the \alert{} command for inline highlighting and block environments like \begin{block}, \begin{alertblock}, and \begin{examples} for distinct content sections. The appearance of these elements depends on the chosen Beamer theme.

```latex
\begin{frame}
\frametitle{Sample frame title}

In this slide, some important text will be
\alert{highlighted} because it's important.
Please, don't abuse it.

\begin{block}{Remark}
Sample text
\end{block}

\begin{alertblock}{Important theorem}
Sample text in red box
\end{alertblock}

\begin{examples}
Sample text in green box. The title of the block is ``Examples".
\end{examples}
\end{frame}
```

--------------------------------

### Example of \jobname expansion

Source: https://www.overleaf.com/learn/latex/Articles/How_does_%5Cexpandafter_work%3A_TeX_uses_temporary_token_lists

Demonstrates how the \jobname primitive expands to the current TeX file's name, including the space after it. This shows TeX's temporary token list usage.

```tex
    The name of my file is \jobname .tex %Note the space after \jobname
```

--------------------------------

### LuaTeX Command Token Calculation Example: \\

Source: https://www.overleaf.com/learn/latex/Articles/An_Introduction_to_LuaTeX_%28Part_2%29%3A_Understanding_%5Cdirectlua

Demonstrates the calculation of a command token for a control symbol '\\'. It shows the intermediate `curcs` value and the final token value.

```lua
for the `\\` command (a control symbol), LuaTeX calculates curcs=94, resulting in a token value for `\\` of 94+229−1=536871005.
```

--------------------------------

### Formatting LaTeX Index with Columns and Title

Source: https://www.overleaf.com/learn/latex/Indices

This example demonstrates how to customize the appearance of the index by specifying the number of columns and the title. These options are passed as parameters to the \makeindex command.

```latex
\documentclass{article}
\usepackage[T1]{fontenc}
\usepackage{imakeidx}
\makeindex[columns=3, title=Alphabetical Index]

\begin{document}
\section{Introduction}
In this example, several keywords\index{keywords} will be used which 
are important and deserve to appear in the Index\index{Index}.

Terms like generate\index{generate} and some\index{others} will also 
show up. Terms in the index can also be nested \index{Index!nested}

\clearpage

\section{Second section}
This second section\index{section} may include some special word, 
and expand the ones already used\index{keywords!used}.

\printindex
\end{document}

```

--------------------------------

### Basic pTeX Document Structure

Source: https://www.overleaf.com/learn/latex/Latex-questions/Does_Overleaf_support_pTeX%3F

A simple example of a pTeX document using the `jsarticle` class, suitable for Japanese text.

```latex
\documentclass{jsarticle}

\bibliographystyle{jplain}
\title{p\LaTeX\ 実験}
\author{林蓮枝}

\begin{document}

\maketitle

\begin{abstract}
本稿では、文書組版システムp\LaTeX{}の使い方を解説します。p\LaTeX{}を利用するときには、あらかじめ文章中に\TeX{}コマンドと呼ばれる組版用の指示を混在させ\ldots
\end{abstract}

\section{導入}
こんにちは世界！

\end{document}
```

--------------------------------

### Configuring biblatex with Options

Source: https://www.overleaf.com/learn/latex/Basic_bibliography_management

This example shows how to configure the biblatex package with various options passed during the \usepackage command. Options include setting the backend, bibliography style, and sorting criteria.

```latex
\documentclass{article}

\usepackage[
backend=biber,
style=alphabetic,
sorting=ynt
]{biblatex}
\addbibresource{sample.bib}

\title{Bibliography management: \texttt{biblatex} package}
\author{Overleaf}
\date{ }

\begin{document}

\maketitle

Using \texttt{biblatex} you can display a bibliography divided 
into sections, depending on citation type. Let's cite! Einstein's 
journal paper \cite{einstein} and Dirac's book \cite{dirac} are 
physics-related items. Next, \textit{The \LaTeX\ Companion} book
 \cite{latexcompanion}, Donald Knuth's website \cite{knuthwebsite},
\textit{The Comprehensive Tex Archive Network} (CTAN) 
\cite{ctan} are \LaTeX-related items; but the others, Donald Knuth's items, 
\cite{knuth-fa,knuth-acp} are dedicated to programming. 

\medskip

\printbibliography

\end{document}

```

--------------------------------

### Example of Printing Counter Values

Source: https://www.overleaf.com/learn/latex/Counters

Demonstrates the usage of various commands to format counter values, including arabic, roman, Roman, alph, Alph, and fnsymbol.

```latex
\newcounter{somecounter}
\setcounter{somecounter}{9}
\begin{itemize}
    \item \verb|\arabic{somecounter}| typesets the \texttt{somecounter} value of  \thesomecounter{} as \arabic{somecounter} 
    \item \verb|\roman{somecounter}| typesets the \texttt{somecounter} value of  \thesomecounter{} as  \roman{somecounter}
    \item \verb|\Roman{somecounter}| typesets the \texttt{somecounter} value of  \thesomecounter{} as  \Roman{somecounter}
    \item \verb|\alph{somecounter}| typesets the \texttt{somecounter} value of  \thesomecounter{} as  \alph{somecounter}
   \item \verb|\Alph{somecounter}| typesets the \texttt{somecounter} value of  \thesomecounter{} as  \Alph{somecounter}
    \item \verb|\fnsymbol{somecounter}| typesets the \texttt{somecounter} value of  \thesomecounter{} as  \fnsymbol{somecounter}
\end{itemize}


```

--------------------------------

### Highlighting Text with lua-ul Package

Source: https://www.overleaf.com/learn/latex/Using_colours_in_LaTeX

This example demonstrates the \highLight command from the lua-ul package for highlighting text with specified or default colors. It requires the luacolor and xcolor packages.

```latex
\documentclass{article}
% Prefer a small page width for the demo
\usepackage[paperwidth=12cm]{geometry}
  
\usepackage[dvipsnames]{xcolor} % To access some named colors used with \highLight
\usepackage{luacolor} % Required to use the lua-ul \highLight command 
\usepackage{lua-ul} 
  
\usepackage{blindtext}
\begin{document}
% Use the Apricot color to highlight the text  
\highLight[Apricot]{\blindtext} 

% Use \LuaULSetHighLightColor to set default colors for 
% the \highLight command
\begin{itemize}
    \item \LuaULSetHighLightColor{CornflowerBlue} \highLight{\blindtext[1]}.
    \item \LuaULSetHighLightColor{Goldenrod}\highLight{{\(y=f(x)\)}}
\end{itemize}
\end{document}
```

--------------------------------

### Enable SyncTeX with uncompressed output

Source: https://www.overleaf.com/learn/latex/MLTeX_SyncTeX_and_EncTeX_TeX_extensions

Use the -synctex=-1 option to enable SyncTeX and generate an uncompressed .synctex output file.

```bash
pdftex -synctex=-1 myfile.tex

```

--------------------------------

### Example LaTeX Preamble

Source: https://www.overleaf.com/learn/latex/Errors/%3ALaTeX_Error%3A_Can_be_used_only_in_preamble

A typical LaTeX preamble includes document class, input encoding, and package inclusions. Ensure all \usepackage commands are within this section.

```latex
\documentclass[12pt, letterpaper]{article}
\usepackage[utf8]{inputenc}
\usepackage{amsmath}

\title{First document}
\author{Hubert Farnsworth \thanks{funded by the ShareLaTeX team}}
\date{February 2014}


```

--------------------------------

### Display Style Math with \displaystyle Override

Source: https://www.overleaf.com/learn/latex/Display_style_in_math_mode

This example demonstrates how to use the \displaystyle command to force elements within a fraction to be typeset in display style, affecting the layout and size of symbols and limits.

```latex
\[
a_0+{1\over\displaystyle a_1+
      {1\over\displaystyle a_2+
        {1 \over\displaystyle a_3 + 
           {1 \over\displaystyle a_4}}}}
\]

```

--------------------------------

### Convert EPS to PDF using epstopdf

Source: https://www.overleaf.com/learn/how-to/Optimising_very_large_image_files

Use the epstopdf command to convert EPS files to PDF. Ensure GhostScript is installed.

```bash
gs -o outputfile.eps -sDEVICE=epswrite originalfile.eps
```

--------------------------------

### LuaTeX Command Token Calculation Example: \vskip

Source: https://www.overleaf.com/learn/latex/Articles/An_Introduction_to_LuaTeX_%28Part_2%29%3A_Understanding_%5Cdirectlua

Illustrates the calculation of a command token for the primitive command '\vskip'. It includes the intermediate `curcs` value and the resulting token value.

```lua
for the `\vskip` primitive command (a control word) LuaTeX calculates curcs=3560, resulting in a token value for `\vskip` of 3560+229−1=536874471.
```

--------------------------------

### Example: Token for Space Character

Source: https://www.overleaf.com/learn/latex/Articles/What_is_a_%22TeX_token%22%3F

Demonstrates the calculation of a TeX token for a space character with a category code of 10 and ASCII value of 32.

```tex
256*10 + 32 = 2592
```

--------------------------------

### Basic LaTeX Bibliography with thethebibliography Environment

Source: https://www.overleaf.com/learn/latex/Bibliography_management_with_bibtex

Use this environment to manually create a bibliography list in LaTeX. Each entry is defined with \texttt{\bibitem}, which takes a cite key as a parameter. Ensure the argument to \texttt{\begin{thebibliography}} is wide enough for all labels.

```latex
\begin{thebibliography}{9}
\bibitem{texbook}
Donald E. Knuth (1986) \emph{The \TeX{} Book}, Addison-Wesley Professional.

\bibitem{lamport94}
Leslie Lamport (1994) \emph{\LaTeX: a document preparation system}, Addison
Wesley, Massachusetts, 2nd ed.
\end{thebibliography}

```

--------------------------------

### LaTeX Document Structure with Chapters and Sections

Source: https://www.overleaf.com/learn/latex/Creating_a_document_in_LaTeX

Shows how to structure a document using chapters, sections, and subsections with the 'book' document class. Includes an example of an unnumbered section.

```latex
\documentclass{book}
\begin{document}

\chapter{First Chapter}

\section{Introduction}

This is the first section.

Lorem  ipsum  dolor  sit  amet,  consectetuer  adipiscing  
elit. Etiam  lobortisfacilisis sem.  Nullam nec mi et 
neque pharetra sollicitudin.  Praesent imperdietmi nec ante. 
Donec ullamcorper, felis non sodales...

\section{Second Section}

Lorem ipsum dolor sit amet, consectetuer adipiscing elit.  
Etiam lobortis facilisissem.  Nullam nec mi et neque pharetra 
solicitudin.  Praesent imperdiet mi necante...

\subsection{First Subsection}
Praesent imperdietmi nec ante. Donec ullamcorper, felis non sodales...

\section*{Unnumbered Section}
Lorem ipsum dolor sit amet, consectetuer adipiscing elit.  
Etiam lobortis facilisissem...
\end{document}
```

--------------------------------

### LaTeX Document Preamble Configuration

Source: https://www.overleaf.com/learn/latex/Creating_a_document_in_LaTeX

The preamble, located before \begin{document}, sets up the document's properties. This example shows how to define document class options like font size and paper size, and how to load packages.

```latex
\documentclass[12pt, letterpaper]{article}
```

```latex
\usepackage{graphicx}
```

--------------------------------

### Picture with Offset Origin

Source: https://www.overleaf.com/learn/latex/Picture_environment

Example of a picture environment with a width and height of 3 units, with an origin offset of (1,1).

```latex
\begin{picture}(3,3)(1,1)
... 
\end{picture}
```

--------------------------------

### Using the 'tabular' Environment

Source: https://www.overleaf.com/learn/latex/Environments%23Defining_a_new_environment

Shows how to use the 'tabular' environment to typeset a table with specified column alignment.

```latex
\documentclass{article}
\begin{document}
\begin{tabular}{ c c c } 
  cell1 & cell2 & cell3 \ 
  cell4 & cell5 & cell6 \ 
  cell7 & cell8 & cell9 \ 
 \end{tabular}
\end{document}
```

--------------------------------

### Display LuaTeX Help Information

Source: https://www.overleaf.com/learn/latex/Articles/The_two_modes_of_TeX_engines%3A_INI_mode_and_production_mode

Use the `--help` option to view all supported command-line options for the LuaTeX engine. This is useful for understanding available modes and configurations.

```bash
luatex --help
```

--------------------------------

### LuaLaTeX document with luatex-ja package

Source: https://www.overleaf.com/learn/latex/Japanese%23The_pTeX_engine

Example of a Japanese document using the `scrartcl` class and loading the `luatex-ja` package for typesetting.

```latex
\documentclass{scrartcl}
\usepackage{luatexja}
\begin{document}
\section{これは最初のセクションである}
日本語で \LaTeX の組版を実証するための導入部分。

フォントはまた、数学的な形態および他の環境で使用することができる
\end{document}
```

--------------------------------

### LaTeX: Fixed \baselineskip example

Source: https://www.overleaf.com/learn/latex/Articles/How_to_change_paragraph_spacing_in_LaTeX

Demonstrates setting a fixed value for \baselineskip to control paragraph line spacing. Change the \baselineskip value to observe its effect on the output.

```latex
\documentclass{article}
\title{Demonstrating baselineskip}
% Choose a conveniently small page size
\usepackage[paperheight=16cm,paperwidth=12cm,textwidth=8cm]{geometry}
% Set the value of some paragraph-related parameters (we will discuss these)
\setlength{\lineskip}{3.5pt}
\setlength{\lineskiplimit}{2pt}
\setlength{\parindent}{20pt}
% Set the value of \baselineskip---change the value to see the effect
\setlength{\baselineskip}{12pt}

\begin{document}
\input text.tex % A generated TeX file which defines the macros
% \mytextA and \mytextB, each of which typeset a paragraphs

% Firstly, typeset two paragraphs with the default \baselineskip
\mytextA\mytextB

% Now change \baselineskip to a larger value and typeset
% another paragraph
\setlength{\baselineskip}{24pt}
\mytextA

\end{document}

```

--------------------------------

### LuaLaTeX Example with COLR/CPAL Font (Vector Duck)

Source: https://www.overleaf.com/learn/latex/Articles/An_overview_of_technologies_supporting_the_use_of_colour_emoji_fonts_in_LaTeX

This LuaLaTeX example demonstrates typesetting a vector emoji duck using TwemojiMozilla.ttf. It utilizes `Renderer=Harfbuzz` and `SizeFeatures={Size=400}` for scalable rendering of the emoji.

```latex
\documentclass{article}
\usepackage{fontspec}
\title{Duck demo}
\begin{document}
\newfontfamily\emojifont[Renderer=Harfbuzz,SizeFeatures={Size=400}]{TwemojiMozilla.ttf}
\emojifont\Uchar"1F986
\end{document}

```

--------------------------------

### Include Package with Option

Source: https://www.overleaf.com/learn/latex/Writing_your_own_package

This is the command used in a LaTeX document to load the 'examplepackage' and apply the 'red' option, which changes the color of important words.

```tex
\usepackage[red]{examplepackage}
```

--------------------------------

### Horizontal Glue with Different Units

Source: https://www.overleaf.com/learn/latex/Articles/How_TeX_Calculates_Glue_Settings_in_an_%5Chbox

Provides examples of using \hskip with various physical units for natural width, stretch, and shrink components, including mm, in, cm, and pt.

```tex
\hskip 3mm plus 2mm minus 1mm
```

```tex
\hskip 3in plus 2in minus 1in
```

```tex
\hskip 1in plus 3cm minus 20mm
```

--------------------------------

### Picture with Default Origin

Source: https://www.overleaf.com/learn/latex/Picture_environment

Example of a picture environment with a width and height of 3 units, using the default origin (0,0).

```latex
\begin{picture}(3,3)
... 
\end{picture}
```

--------------------------------

### Example of an unnumbered remark environment in LaTeX

Source: https://www.overleaf.com/learn/latex/Theorems_and_proofs

Shows how to use an unnumbered 'remark' environment defined with \newtheorem*. This environment is useful for adding supplementary information without affecting document numbering.

```latex
\documentclass{article}
\usepackage[english]{babel}
\usepackage{amsthm}

\newtheorem*{remark}{Remark}

\begin{document}
Unnumbered theorem-like environments are also possible.

\begin{remark}
This statement is true, I guess.
\end{remark}
\end{document}
```

--------------------------------

### LuaTeX Token List Example

Source: https://www.overleaf.com/learn/latex/Articles/An_Introduction_to_LuaTeX_%28Part_2%29%3A_Understanding_%5Cdirectlua

Illustrates the internal representation of a token list generated by LuaTeX from the input 'Hi, \TeX! \hskip 5bp'. This shows the sequence of nodes, each containing a token value and a pointer to the next node.

```text
Node Address | Token Value | What each token means
------------------------------------------------------------------
0x12345678 | 23068744 | H (character, catcode 11)
0x12345690 | 23068777 | i (character, catcode 11)
0x123456A8 | 25165868 | , (character, catcode 12)
0x123456C0 | 20971552 | <space> (character, catcode 10)
0x123456D8 | 536871539 | \TeX (command, macro)
0x123456F0 | 25165857 | ! (character, catcode 12)
0x12345708 | 20971552 | <space> (character, catcode 10)
0x12345720 | 536874247 | \hskip (command, primitive)
0x12345738 | 25165877 | 5 (character, catcode 12)
0x12345750 | 23068770 | b (character, catcode 11)
0x12345768 | 23068784 | p (character, catcode 11)
0x12345780 | null value | End of list
```

--------------------------------

### Define an unnumbered remark environment in LaTeX

Source: https://www.overleaf.com/learn/latex/Theorems_and_proofs

Use \newtheorem* to create an unnumbered theorem-like environment. This is useful for remarks, comments, or examples that do not require numbering.

```latex
\newtheorem*{remark}{Remark}
```

--------------------------------

### Demonstrate Page Numbering Styles

Source: https://www.overleaf.com/learn/latex/Page_numbering

This example showcases different page numbering styles including Roman, Arabic, and alphabetic numerals. It resets the page numbering style for each section.

```latex
\documentclass{article}
\usepackage{blindtext}
\begin{document}
% Insert a table of contents
\tableofcontents
\newpage
\section{Uppercase Roman}
\pagenumbering{Roman}% Capital 'R': uppercase Roman numerals
\blindtext[1]
\newpage
\section{Lowercase Roman} % lowercase Roman numerals
\pagenumbering{roman}
\blindtext[1]
\newpage
\section{Arabic numbers}
\pagenumbering{arabic} % Arabic/Indic page numbers
\blindtext[1]
\newpage
\section{Lowercase alphabetic}
\pagenumbering{alph} % Lowercase alphabetic page "numbers"
\blindtext[1]
\newpage
\section{Uppercase alphabetic}
\pagenumbering{Alph} % Uppercase alphabetic page "numbers"
\blindtext[1]
\end{document}
```

--------------------------------

### Grouping Nomenclature Entries with etoolbox

Source: https://www.overleaf.com/learn/latex/Nomenclatures

This example illustrates how to group nomenclature entries by type (e.g., Physics constants, Number sets) using the `etoolbox` package and the `\nomgroup` command.

```latex
\documentclass{article}
\usepackage{amssymb}
\usepackage{nomencl}
\makenomenclature

%% This code creates the groups
% -----------------------------------------
\usepackage{etoolbox}
\renewcommand\nomgroup[1]{\item[\bfseries
  \ifstrequal{#1}{P}{Physics constants}{
  \ifstrequal{#1}{N}{Number sets}{
  \ifstrequal{#1}{O}{Other symbols}{}}}
]}
% -----------------------------------------

\begin{document}
Here is an example:

\nomenclature[P]{\(c\)}{Speed of light in a vacuum}
\nomenclature[P]{\(h\)}{Planck constant}
\nomenclature[P]{\(G\)}{Gravitational constant}
\nomenclature[N]{\(\mathbb{R}\)}{Real numbers}
\nomenclature[N]{\(\mathbb{C}\)}{Complex numbers}
\nomenclature[N]{\(\mathbb{H}\)}{Quaternions}
\nomenclature[O]{\(V\)}{Constant volume}
\nomenclature[O]{\(\rho\)}{Friction index}

\printnomenclature
\end{document}
```

--------------------------------

### Defining a Macro in TeX

Source: https://www.overleaf.com/learn/latex/Articles/What_is_a_%22TeX_token%22%3F

Example of a simple macro definition in TeX. This is used to illustrate how TeX processes input to create tokens.

```tex
\def\ohyeah{Overleaf is cool!}

```

--------------------------------

### Demonstrate \baselinestretch and \selectfont

Source: https://www.overleaf.com/learn/latex/Articles/How_to_change_paragraph_spacing_in_LaTeX

This example shows how \baselinestretch affects \baselineskip. Initially, \baselinestretch is updated but \baselineskip remains unchanged. The \selectfont command is then used to apply the \baselinestretch change, updating \baselineskip accordingly. It also demonstrates how changing font size with \fontsize interacts with \baselinestretch.

```latex
The default value of \verb|\baselineskip|=\the\baselineskip. Now update \verb|\baselinestretch|\renewcommand\baselinestretch{2.5}. However, although we have executed

\vspace{5pt}
\verb|\renewcommand\baselinestretch{2.5}|
\vspace{5pt}

\noindent it \textit{has not} (yet) had any effect: the new value of \verb|\baselinestretch| is stored, but not actioned. We can issue a \verb|\selectfont| command to make the change take effect; but, for now, the value of \verb|\baselineskip| is still \the\baselineskip{} (unchanged). 

If we now execute \verb|\selectfont| the change to \verb|\baselinestretch|, setting it to 2.5, will now take effect---\selectfont even though we are in the middle of a paragraph. Now, the value of \verb|\baselineskip| is \the\baselineskip{} (= \(2.5 \times 12=30\mathrm{pt}\)).

What happens if we change the font size to 14pt and \verb|\baselineskip| to 16pt using the commands \verb|\fontsize{14}{16}\selectfont|? 

\fontsize{14}{16}\selectfont Now, \verb|\baselineskip| is \textit{not} 16pt but \the\baselineskip{} because the value of  \verb|\baselinestretch| (2.5) has been preserved and applied, giving \verb|\baselineskip| = \(2.5 \times 16=40\mathrm{pt}\).

What happens if we change the font size to 14pt and \verb|\baselineskip| to 16pt using the commands \verb|\fontsize{14}{16}\selectfont|? 

\fontsize{14}{16}\selectfont Now, \verb|\baselineskip| is \textit{not} 16pt but \the\baselineskip{} because the value of  \verb|\baselinestretch| (2.5) has been preserved and applied, giving \verb|\baselineskip| = \(2.5 \times 16=40\mathrm{pt}\).
```

--------------------------------

### Use a defined theorem environment in LaTeX

Source: https://www.overleaf.com/learn/latex/Theorems_and_proofs

Once defined, environments like 'theorem' can be used with \begin{theorem} and \end{theorem}. This example shows a simple theorem within a document.

```latex
\documentclass{article}
\usepackage[english]{babel}
\newtheorem{theorem}{Theorem}
\begin{document}

\section{Introduction}
Theorems can easily be defined:

\begin{theorem}
Let \(f\) be a function whose derivative exists in every point, then \(f\) 
is a continuous function.
\end{theorem}
\end{document}
```

--------------------------------

### Import Code from File

Source: https://www.overleaf.com/learn/latex/Code_listing

Use \lstinputlisting to import code directly from a file. Specify the language for syntax highlighting.

```LaTeX
\lstinputlisting[language=Octave]{BitXorMatrix.m}
```

--------------------------------

### Integral without Limits

Source: https://www.overleaf.com/learn/latex/Subscripts_and_superscripts

This example shows a definite integral without the `\limits` command, causing the limits to appear next to the integral symbol rather than above and below.

```latex
\[ 
  \int_0^1 x^2 + y^2 \ dx 
\]
```

--------------------------------

### Visualize LaTeX Document Layout

Source: https://www.overleaf.com/learn/latex/Single_sided_and_double_sided_documents

This example uses the `layout` package to visualize the default page layout for different document classes. Uncomment the desired `\documentclass` line to explore its layout. The `twoside` option affects the output for classes like `article` and `report`.

```latex
% Choose the document class whose layout you want to visualize: uncomment
% the one you want, comment out the others.
% \documentclass[a4paper]{article} %Produces one page (based on A4 paper size)
% \documentclass[a4paper]{report} %Produces one page (based on A4 paper size)
% \documentclass[twoside,a4paper]{report} %Produces two pages (based on A4 paper size)
% \documentclass[a4paper]{book} %Produces two pages (based on A4 paper size)
% \documentclass[a4paper]{letter} %Produces one page (based on A4 paper size)
% \documentclass[twoside, a4paper]{letter} %Produces two pages (based on A4 paper size)
\documentclass[twoside,a4paper]{article} %Produces two pages (based on A4 paper size)
\usepackage{layout}
\begin{document}
\layout
\end{document}
```

--------------------------------

### Combining Grouping and Manual Sorting in Nomenclatures

Source: https://www.overleaf.com/learn/latex/Nomenclatures

This example shows how to combine the subgrouping functionality with manual sorting using numerical prefixes for nomenclature entries.

```latex
\documentclass{article}
\usepackage{amssymb}
\usepackage{nomencl}
\makenomenclature

\usepackage{etoolbox}
\renewcommand\nomgroup[1]{\item[\bfseries
  \ifstrequal{#1}{A}{Physics Constants}{
  \ifstrequal{#1}{B}{Number Sets}{
  \ifstrequal{#1}{C}{Other Symbols}}}
]}

\begin{document}
Here is an example:

\nomenclature[A, 02]{\(c\)}{Speed of light in a vacuum}
\nomenclature[A, 03]{\(h\)}{Planck constant}
\nomenclature[A, 01]{\(G\)}{Gravitational constant}
\nomenclature[B, 03]{\(\mathbb{R}\)}{Real numbers}
\nomenclature[B, 02]{\(\mathbb{C}\)}{Complex numbers}
\nomenclature[B, 01]{\(\mathbb{H}\)}{Octonions}
\nomenclature[C]{\(V\)}{Constant volume}
\nomenclature[C]{\(\rho\)}{Friction index}

\printnomenclature

\end{document}
```

--------------------------------

### Feynman Diagram with Scalar and Photon Edges

Source: https://www.overleaf.com/learn/latex/Feynman_diagrams

Illustrates a basic Feynman diagram with scalar and photon edges, using the `small` layout option and `horizontal` alignment. This example is used to demonstrate invisible edges.

```latex
% No invisible to keep the two photons together
\feynmandiagram [small, horizontal=a to t1] {
  a [particle=\(\pi^{0}\)] -- [scalar] t1 -- t2 -- t3 -- t1,
  t2 -- [photon] p1 [particle=\(\gamma\)],
  t3 -- [photon] p2 [particle=\(\gamma\)],
};

```

--------------------------------

### Beamerposter Body Content Structure

Source: https://www.overleaf.com/learn/latex/Posters

Structure the content of a beamerposter within a frame environment. Utilize block environments for titled sections and columns for multi-column layouts. This example shows basic text, font size examples, and itemized lists.

```latex
\documentclass{beamer}
  \usepackage{times}
  \usepackage{amsmath,amsthm, amssymb}
  \boldmath
  \usetheme{RedLion}
  \usepackage[orientation=portrait,size=a0,scale=1.4]{beamerposter}

  \title[Beamer Poster]{Overleaf example of the beamerposter class}
  \author[welcome@overleaf.com]{Overleaf Team}
  \institute[Overleaf University]
  {The Overleaf institute, Learn faculty}
  \date{\today}
  
  \logo{\includegraphics[height=7.5cm]{overleaf-logo}}

  \begin{document}
  \begin{frame}{} 
    \vfill
    \begin{block}{\large Fontsizes}
      \centering
      {\tiny tiny}\par
      {\scriptsize scriptsize}\par
      {\footnotesize footnotesize}\par
      {\normalsize normalsize}\par
      ...
    \end{block}
    
    \end{block}
    \vfill
    \begin{columns}[t]
      \begin{column}{.30\linewidth}
        \begin{block}{Introduction}
          \begin{itemize}
          \item some items
          \item some items
          ...
          \end{itemize}
        \end{block}
      \end{column}
      \begin{column}{.48\linewidth}
        \begin{block}{Introduction}
          \begin{itemize}
          \item some items and $\alpha=\gamma, \sum_{i}$
          ...
          \end{itemize}
          $$\alpha=\gamma, \sum_{i}$$
        \end{block}
        ...

      \end{column}
    \end{columns}
  \end{frame}
\end{document}


```

--------------------------------

### Basic Table Positioning in LaTeX

Source: https://www.overleaf.com/learn/latex/Positioning_images_and_tables

The tabular environment defaults to center alignment. This example demonstrates a basic table structure with colored rows and columns.

```latex
Praesent in sapien. Lorem ipsum dolor sit amet, consectetuer adipiscing elit. Duis fringilla tristique neque. 
Sed interdum libero ut metus. Pellentesque placerat. Nam rutrum augue a leo. Morbi sed elit sit amet 
ante lobortis sollicitudin.

\arrayrulecolor[HTML]{DB5800}
\begin{tabular}{ |s|p{2cm}|p{2cm}|  }
\hline
\rowcolor{lightgray} \multicolumn{3}{|c|}{Country List} \\
\hline
Country Name or Area Name& ISO ALPHA 2 Code &ISO ALPHA 3 \\
\hline
Afghanistan & AF &AFG \\
\rowcolor{gray}
Aland Islands & AX  & ALA \\
Albania    &AL & ALB \\
Algeria   &DZ & DZA \\
American Samoa & AS & ASM \\
Andorra & AD & \cellcolor[HTML]{AA0044} AND \\
Angola & AO & AGO \\
\hline
\end{tabular}

Praesent in sapien. Lorem ipsum dolor sit amet, consectetuer adipiscing 
elit. Duis fringilla tristique neque. Sed interdum libero ut metus. 
Pellentesque placerat. Nam rutrum augue a leo. Morbi sed elit sit 
amet ante lobortis sollicitudin.

```

--------------------------------

### Example Using Named Colors with xcolor

Source: https://www.overleaf.com/learn/latex/Using_colours_in_LaTeX

Illustrates using named colors like ForestGreen and RubineRed with the xcolor package, including applying colors to list items, horizontal rules, and text backgrounds.

```latex
\documentclass{article}
\usepackage[dvipsnames]{xcolor}
\begin{document}
This example shows how to use the \texttt{xcolor} package 
to change the color of \LaTeX{} page elements.

\begin{itemize}
\color{ForestGreen}
\item First item
\item Second item
\end{itemize}

\noindent
{\color{RubineRed} \rule{\linewidth}{0.5mm}}

The background color of text can also be \textcolor{red}{easily} set. For 
instance, you can change use an \colorbox{BurntOrange}{orange background} and then continue typing.
\end{document}
```

--------------------------------

### LaTeX Error Message Example

Source: https://www.overleaf.com/learn/latex/Errors/%3ALaTeX_Error%3A_Can_be_used_only_in_preamble

This error message indicates that a \usepackage command was found after \begin{document}. Corrective action involves moving such commands to the preamble.

```latex
main.tex, line 7
LaTeX Error: Can be used only in preamble.
See the LaTeX manual or LaTeX Companion for explanation. Type H <return> for immediate help. ... l.4 \usepackage {amsmath} Your command was ignored. Type I <command> <return> to replace it with another command, or <return> to continue without it. [1 

```

--------------------------------

### pdfLaTeX Example for Russian Text

Source: https://www.overleaf.com/learn/latex/Russian

Use this example to enable Russian language support in pdfLaTeX. It requires the `fontenc` package with the `T2A` option for Cyrillic encoding and the `babel` package with the `russian` option. Custom hyphenation rules can be defined using the `hyphenat` package.

```latex
\documentclass{article}
\usepackage[T2A]{fontenc}

%Hyphenation rules
%--------------------------------------
\usepackage{hyphenat}
\hyphenation{ма-те-ма-ти-ка вос-ста-нав-ли-вать}
%--------------------------------------
\usepackage[english, russian]{babel}
\begin{document}
 
\tableofcontents

\begin{abstract}
  Это вводный абзац в начале документа.
\end{abstract}
 
\section{Предисловие}
 Этот текст будет на русском языке. Это демонстрация того, что символы кириллицы
 в сгенерированном документе (Compile to PDF) отображаются правильно. Для этого Вы должны установить нужный  язык (russian) и необходимую кодировку шрифта (T2A).

\vskip12pt

\textbf{Этот текст будет на русском языке. Это демонстрация того, что символы кириллицы в сгенерированном документе (Compile to PDF) отображаются правильно.}

\vskip12pt

\textit{Этот текст будет на русском языке. Это демонстрация того, что символы кириллицы в сгенерированном документе (Compile to PDF) отображаются правильно.} 

\section{Математические формулы}
Кириллические символы также могут быть использованы в математическом режиме.

\begin{equation}
  S_\textup{ис} = S_{123}
\end{equation}
\end{document}
```

--------------------------------

### Basic German Typesetting with pdfLaTeX

Source: https://www.overleaf.com/learn/latex/German

Use this example to enable correct typesetting of German characters and hyphenation. Ensure T1 font encoding is set and the 'ngerman' option for babel is included.

```latex
\documentclass{article}

% \usepackage[utf8]{inputenc} is no longer required (since 2018)

%Set the font (output) encoding
%--------------------------------------
\usepackage[T1]{fontenc} %Not needed by LuaLaTeX or XeLaTeX
%--------------------------------------

%German-specific commands
%--------------------------------------
\usepackage[ngerman]{babel}

%Hyphenation rules
%--------------------------------------
\usepackage{hyphenat}
\hyphenation{Mathe-matik wieder-gewinnen}
%--------------------------------------
\begin{document}
\tableofcontents
\vspace{2cm} %Add a 2cm space

\begin{abstract}
Dies ist eine kurze Zusammenfassung der Inhalte des in deutscher Sprache
verfassten Dokuments.
\end{abstract}

\section{Einleitendes Kapitel}
Dies ist der erste Abschnitt. Hier können wir einige zusätzliche Elemente
hinzufügen und alles wird korrekt geschrieben und umgebrochen werden. Falls ein
Wort für eine Zeile zu lang ist, wird \texttt{babel} versuchen je nach Sprache
richtig zu trennen.

\section{Eingabe mit mathematischer Notation}
In diesem Abschnitt ist zu sehen, was mit Makros geschieht, die zuvor definiert wurden.

\[ \lim x =  \theta + 152383.52 \]
\end{document}
```

--------------------------------

### \subimport Path Example

Source: https://www.overleaf.com/learn/latex/Management_in_a_large_project%23Importing_files

Demonstrates the usage of \subimport to include a file ('plot1.tex') located in a subdirectory ('img/'). The path is relative to the current file ('section1-1.tex').

```latex
\subimport{img/}{plot1.tex}
```

--------------------------------

### Set Legal Paper Size, Landscape Orientation, and Margins using \geometry

Source: https://www.overleaf.com/learn/latex/Page_size_and_margins

Achieve the same layout as the previous example (legal paper, landscape orientation, 2-inch margin) by first loading the geometry package and then using the \geometry command in the preamble.

```latex
\usepackage{geometry}
\geometry{legalpaper, landscape, margin=2in}
```

--------------------------------

### Modify `enumerate` List Starting Number

Source: https://www.overleaf.com/learn/latex/Counters%23Introduction_to_LaTeX_counters

Shows how to change the starting number of an `enumerate` list by using `\setcounter{enumi}{3}`. This sets the initial value of the `enumi` counter (used for the first level of `enumerate`) to 3, so the first item will be numbered '4'.

```latex
This example shows one way to change the numbering of a list; here, changing the value of the \texttt{enumi} counter to start the list numbering at 4 (it is incremented by the \verb|\item| command):

\begin{enumerate}
\setcounter{enumi}{3}
\item Something.
\item Something else.
\item Another element.
\item The last item in the list.
\end{enumerate}
```

--------------------------------

### Basic \expandafter Expansion

Source: https://www.overleaf.com/learn/latex/Articles/How_does_%5Cexpandafter_work%3A_A_detailed_study_of_consecutive_%5Cexpandafter_commands

Demonstrates the fundamental expansion of \expandafter on a sequence of commands. This is useful for understanding the order of operations in token expansion.

```latex
\expandafter\expandafter\expandafter\foo\bar
```

--------------------------------

### Define a Simple TeX Macro

Source: https://www.overleaf.com/learn/latex/Articles/%5Cexpandafter_TeX_tokens?preview=true

Defines a basic TeX macro named \hello which includes text and a horizontal skip command. This serves as an example for tokenization.

```tex
\def\hello{Greetings, from \TeX. \hskip 10pt}
```

--------------------------------

### Default article class headers and footers

Source: https://www.overleaf.com/learn/latex/Headers_and_footers

Demonstrates the default header and footer behavior for the article document class. Includes basic document setup and dummy text.

```latex
\documentclass{article}
% Choose a conveniently small page size
\usepackage[paperheight=16cm,paperwidth=12cm,textwidth=10cm]{geometry}
\usepackage{lipsum}% for some dummy text
\title{An article class example}
\author{Overleaf}
\begin{document}
\maketitle

\section{In the beginning...}
\lipsum[2]

\section{Another section}
\lipsum[1]

\section{Yet another}
\lipsum[1]


```

--------------------------------

### Subscript with Nested Index

Source: https://www.overleaf.com/learn/latex/Subscripts_and_superscripts

Example of a subscript where the index itself is a subscript.

```latex
`a_{n_i}`
```

--------------------------------

### Basic Macros for \edef Example

Source: https://www.overleaf.com/learn/latex/Articles/How_does_%5Cexpandafter_work%3A_A_detailed_macro_case_study

Defines a simple macro \mycount that assigns a value to \count99. This is used to demonstrate how \edef handles assignments during expansion.

```tex
\def\mycount{\count99=12345}
     \edef\mymacro{\mycount}
```

--------------------------------

### LuaLaTeX Example with SVG Font (using fontspec)

Source: https://www.overleaf.com/learn/latex/Articles/An_overview_of_technologies_supporting_the_use_of_colour_emoji_fonts_in_LaTeX

This pseudo-code example shows how to use SVG-based OpenType color fonts with LuaLaTeX via the `fontspec` package. It suggests using `RawFeature={+svg}` and `SizeFeatures={Size=20}`. Replace `your SVG font file name here` with the actual font file name.

```latex
\documentclass{article}
\usepackage{fontspec}
\begin{document}
\newfontfamily\emoji[RawFeature={+svg},SizeFeatures={Size=20}]{your SVG font file name here}
\emoji Your emoji here...
\end{document}

```

--------------------------------

### Customizing AVM Font and Basic Usage

Source: https://www.overleaf.com/learn/latex/Attribute_Value_Matrices

Shows how to change the font for AVM content using \\avmfont and illustrates basic AVM syntax with nested structures. This example requires the 'avm' package.

```latex
\documentclass{article}

\usepackage{avm}
\avmfont{\sc}

\begin{document}
\begin{avm}
    [ subj & [ pers & 3 \\
                 num & sg \\
                 gend & masc\\
                 pred & \rm ‘pro’ ]\\
                                                       
        pred & \rm ‘eat\q<SUBJ, OBJ\q>’\\
                                                                            
                 obj & [ pers & 3 \\
                 num & pl \\
                 gend & fem \\
                 pred & \rm ‘pro’ ]
        ]
\end{avm}

\end{document}

```

--------------------------------

### Demonstrate \chardef non-expandability with \directlua

Source: https://www.overleaf.com/learn/latex/Articles/An_Introduction_to_LuaTeX_%28Part_2%29%3A_Understanding_%5Cdirectlua

This example shows how a control sequence defined with \chardef is not expanded by \directlua. The output in the log file retains the \chardef-defined command literally, demonstrating its non-expandable nature.

```tex
\chardef\mydollar=`\$
\directlua{
   local x =[[I paid \mydollar30.]] 
   texio.write(x)
}

```

--------------------------------

### Define and Print Glossary with Custom Title

Source: https://www.overleaf.com/learn/latex/Glossaries

This example demonstrates how to define glossary entries and print the glossary with a custom title and table of contents entry. Ensure the \makeglossaries command is used.

```latex
\documentclass{article}
\usepackage{glossaries}

\makeglossaries


\newglossaryentry{maths}
{
    name=mathematics,
    description={Mathematics is what mathematicians do}
}

\newglossaryentry{latex}
{
    name=latex,
    description={Is a markup language specially suited for 
scientific documents}
}


\newglossaryentry{formula}
{
    name=formula,
    description={A mathematical expression}
}

\begin{document}

The \Gls{latex} typesetting markup language is specially suitable 
for documents that include \gls{maths}. \Glspl{formula} are rendered 
properly an easily once one gets used to the commands.

\clearpage

\printglossary[title=Special Terms, toctitle=List of terms]

\end{document}
```

--------------------------------

### BibTeX Entry Examples

Source: https://www.overleaf.com/learn/latex/Bibliography_management_with_natbib

Standard BibTeX syntax for defining bibliography entries like articles, books, and miscellaneous items. Ensure correct field formatting and entry types.

```bibtex
@article{einstein,
    author =       "Albert Einstein",
    title =        "{Zur Elektrodynamik bewegter K{"o}rper}. ({German})
        [{On} the electrodynamics of moving bodies]",
    journal =      "Annalen der Physik",
    volume =       "322",
    number =       "10",
    pages =        "891--921",
    year =         "1905",
    DOI =          "http://dx.doi.org/10.1002/andp.19053221004"
}

@book{latexcompanion,
    author    = "Michel Goossens and Frank Mittelbach and Alexander Samarin",
    title     = "The \LaTeX\ Companion",
    year      = "1993",
    publisher = "Addison-Wesley",
    address   = "Reading, Massachusetts"
}

@misc{knuthwebsite,
    author    = "Donald Knuth",
    title     = "Knuth: Computers and Typesetting",
    url       = "http://www-cs-faculty.stanford.edu/\~{}uno/abcde.html"
}

```

--------------------------------

### Basic biblatex Document Setup

Source: https://www.overleaf.com/learn/latex/Bibliography_management_with_biblatex

A minimal LaTeX document structure using the biblatex package. Load biblatex with desired backend, style, and sorting options, and include your bibliography file using \addbibresource.

```latex
\documentclass{article}
\usepackage[utf8]{inputenc}
\usepackage[english]{babel}

\usepackage{comment}

\usepackage[
backend=biber,
style=alphabetic,
sorting=ynt
]{biblatex}
\addbibresource{sample.bib}

\title{Bibliography management: \texttt{biblatex} package}
\author{Overleaf}
\date{ }

\begin{document}

\maketitle

Using \texttt{biblatex} you can display bibliography divided into sections, 
dependent of citation type. 
Let's cite! The Einstein's journal paper \cite{einstein} and the Dirac's 
book \cite{dirac} are physics related items. 
Next, \textit{The \LaTeX\ Companion} book \cite{latexcompanion}, the Donald 
Knuth's website \cite{knuthwebsite}, \textit{The Comprehensive Tex Archive 
Network} (CTAN) \cite{ctan} are \LaTeX\ related items; but the others Donald 
Knuth's items \cite{knuth-fa,knuth-acp} are dedicated to programming. 

\medskip

\printbibliography[title={Whole bibliography}]

\end{document}
```

--------------------------------

### Create Nested Lists in LaTeX

Source: https://www.overleaf.com/learn/latex/Lists

Demonstrates how to create deeply nested enumerated lists in LaTeX. The example shows the default numbering scheme for nested lists.

```latex
\documentclass{article}
\begin{document}
\renewcommand{\labelenumii}{\arabic{enumi}.\arabic{enumii}}
\renewcommand{\labelenumiii}{\arabic{enumi}.\arabic{enumii}.\arabic{enumiii}}
\renewcommand{\labelenumiv}{\arabic{enumi}.\arabic{enumii}.\arabic{enumiii}.\arabic{enumiv}}

\begin{enumerate}
\item One
\item Two
\item Three
\begin{enumerate}
    \item Three point one
    \begin{enumerate}
    \item Three point one, point one
        \begin{enumerate}
        \item Three point one, point one, point one
        \item Three point one, point one, point two
        \end{enumerate}
    \end{enumerate}
\end{enumerate}
\item Four
\item Five
\end{enumerate}

\end{document}

```

--------------------------------

### Basic Biblatex Document Setup

Source: https://www.overleaf.com/learn/latex/Basic_bibliography_management

This LaTeX code sets up a document using the biblatex package. It includes necessary packages, adds the bibliography file, and displays the bibliography with a custom title. Ensure the 'biber' backend is used for biblatex.

```latex
\documentclass{article}

\usepackage[
backend=biber,
style=alphabetic,
sorting=ynt
]{biblatex}
\addbibresource{sample.bib}

\title{Bibliography management: \texttt{biblatex} package}
\author{Overleaf}
\date{May 2021}

\begin{document}

\maketitle

Using \texttt{biblatex} you can display a bibliography divided into sections, 
depending on citation type. Let's cite! Einstein's journal paper \cite{einstein} 
and Dirac's book \cite{dirac} are physics-related items. Next, \textit{The \LaTeX\ Companion} 
book \cite{latexcompanion}, Donald Knuth's website \cite{knuthwebsite}, 
\textit{The Comprehensive Tex Archive Network} (CTAN) \cite{ctan} are 
\LaTeX-related items; but the others, Donald Knuth's items, 
\cite{knuth-fa,knuth-acp} are dedicated to programming. 

\medskip

\printbibliography[title={Whole bibliography}]


```

--------------------------------

### Call LaTeX Fibonacci Command

Source: https://www.overleaf.com/learn/latex/Articles/LaTeX_is_More_Powerful_than_you_Think_-_Computing_the_Fibonacci_Numbers_and_Turing_Completeness

Example of how to invoke the custom \fibonacci command in a LaTeX document to generate a specified number of Fibonacci terms.

```latex
\fibonacci{10}
```

--------------------------------

### Typesetting Chess Moves with xskak

Source: https://www.overleaf.com/learn/latex/Chess_notation

Use the \mainline command to typeset a sequence of chess moves in algebraic notation. Each move-pair is numbered. Requires the xskak package and \newchessgame to start.

```latex
\documentclass{article}
\usepackage{xskak}
\begin{document}
\newchessgame
\mainline{1.e4 e5 2.Nf3 Nc6 3.Bb5 a6}
\showboard % A skak package command. Future examples will use \chessboard[...]
\end{document}
```

--------------------------------

### Using the 'center' Environment

Source: https://www.overleaf.com/learn/latex/Environments%23Defining_a_new_environment

Demonstrates how to use the built-in 'center' environment to center a paragraph of text.

```latex
\documentclass{article}
\begin{document}
\begin{center}
This is a demonstration of the \texttt{center} environment. 
This paragraph of text will be \textit{centred} because it is 
contained within a special environment. Environments provide 
an efficient way to modify blocks of text within your document.
\end{center}
\end{document}
```

--------------------------------

### BibTeX @inproceedings Entry Example

Source: https://www.overleaf.com/learn/latex/Bibliography_management_with_bibtex

Use the @inproceedings entry type for conference proceeding articles. Include fields like 'author', 'title', 'booktitle', and 'year'.

```bibtex
@inproceedings{FosterEtAl:2003,
  author = {George Foster and Simona Gandrabur and Philippe Langlais and Pierre
    Plamondon and Graham Russell and Michel Simard},
  title = {Statistical Machine Translation: Rapid Development with Limited Resources},
  booktitle = {Proceedings of {MT Summit IX}},
  year = {2003},
  pages = {110--119},
  address = {New Orleans, USA},
}


```

--------------------------------

### Demonstrating Various Math Mode Spaces

Source: https://www.overleaf.com/learn/latex/Spacing_in_math_mode

Illustrates the effect of different spacing commands within math mode, including negative and positive spaces, and standard spaces.

```latex
\documentclass{article}
\usepackage{amsmath}
\begin{document}
Spaces in mathematical mode.

\begin{align*}
f(x) &= x^2\! +3x\! +2 \\
f(x) &= x^2+3x+2 \\
f(x) &= x^2\, +3x\, +2 \\
f(x) &= x^2\: +3x\: +2 \\
f(x) &= x^2\; +3x\; +2 \\
f(x) &= x^2\ +3x\ +2 \\
f(x) &= x^2\quad +3x\quad +2 \\
f(x) &= x^2\qquad +3x\qquad +2
\end{align*}
\end{document}
```

--------------------------------

### Create Overfull Horizontal Box in LaTeX

Source: https://www.overleaf.com/learn/latex/%5Chfuzz

This example shows how to create an overfull horizontal box by forcing content into a narrower width and enabling \overfullrule to visualize it.

```latex
\overfullrule=5mm
\hbox to 36pt{Overleaf}
```

--------------------------------

### Handle Non-existent Package Error in LaTeX

Source: https://www.overleaf.com/learn/latex/Creating_a_document_in_LaTeX

This example demonstrates how LaTeX terminates with an error if a specified package file (.sty) cannot be found. Ensure all package names are correct.

```latex
\documentclass[12pt, letterpaper]{article}
\usepackage{somepackage}% a NON-EXISTENT package
\begin{document}
This will fail!
\end{document}
```

--------------------------------

### Creating an Instruction Table

Source: https://www.overleaf.com/learn/latex/Knitting_patterns

The 'pattern' environment is used to create a table for knitting instructions. It accepts two color parameters for alternating row backgrounds to enhance readability.

```latex
\documentclass{knittingpatern}
\definecolor{colour3}{HTML}{99CCFF}
\definecolor{colour5}{HTML}{CCFFCC}

\begin{document}
\begin{pattern}{colour3}{colour5}
Cast on & (st)
Instruction 1 & (st)
Instruction 2 & (st)
Instruction 3 & (st)
Instruction 4 & (st)
Instruction 5 & (st)
Instruction 6 & (st)
Instruction 7 & (st)
Instruction 8 & (st)
Instruction 9 & (st)
Instruction 10 & (st)
\quad\vdots & \quad\vdots
\end{pattern}
\end{document}

```

--------------------------------

### Correct Usage of amsmath align Environment

Source: https://www.overleaf.com/learn/latex/Errors/Missing_%24_inserted

This is the corrected version of the previous example, showing the proper way to write equations within an 'align*' environment without using '$' signs.

```latex
\documentclass{article}
\usepackage{amsmath}
\begin{document}
\begin{align*}
2x - 5y &=  8 \ 
3x + 9y &=  -12
\end{align*}
\end{document}
```

--------------------------------

### Lua Code for # Character Example

Source: https://www.overleaf.com/learn/latex/Articles/An_Introduction_to_LuaTeX_%28Part_2%29%3A_Understanding_%5Cdirectlua

The Lua code generated and executed by the \directlua command when handling the # character.

```lua
local tbl = {}
tbl[1] = "Hello"
tbl[2] = "World"
tex.print("Table length is "..#tbl)
```

--------------------------------

### Importing a file with \import

Source: https://www.overleaf.com/learn/latex/Management_in_a_large_project

The \import command takes two arguments: the directory path (relative or absolute) and the filename. This example shows importing 'title.tex' from the current directory.

```latex
\import{./}{title.tex}
```

--------------------------------

### Define a New Environment and Command

Source: https://www.overleaf.com/learn/latex/Writing_your_own_package

This code defines a new LaTeX environment 'example' and a command '\important'. The environment creates a numbered section, and the command formats text in a specific color and adds it to the index. Use this for custom document structures and highlighted content.

```tex
%%Numbered environment
\newcounter{example}[section]
\newenvironment{example}[1][]
{\refstepcounter{example}\par\medskip
\noindent \textbf{My~environment~	heexample. #1} \rmfamily}{\medskip}

%%Important words are added to the index and printed in different colour
\newcommand{\important}[1]
{\IfSubStr{#1}{!}
    {\textcolor{\wordcolour}{\textbf{\StrBefore{#1}{!}~\StrBehind{#1}{!}}}\index{#1}}
    {\textcolor{\wordcolour}{\textbf{#1}}\index{#1}\kern-1pt}
}
```

--------------------------------

### Defining Atoms with Orbital Specifications

Source: https://www.overleaf.com/learn/latex/Molecular_orbital_diagrams

This example shows how to define two atoms with detailed atomic orbital specifications, including energy levels and electron spins. It illustrates the syntax for `sub-level = {energy; specifications}`.

```latex
\documentclass{article}
\usepackage{modiagram}
\begin{document}

\begin{modiagram}
 \atom{right}{
    1s = { 0; pair} ,
    2s = { 1; pair} ,
    2p = {1.5; up, down }
 }

 \atom{left}{
    1s = { 0; pair} ,
    2s = { 1; pair} ,
    2p = {1.5; up, down }
 }
 \end{modiagram}
\end{document}
```

--------------------------------

### Writing to Log File with \directlua and texio.write

Source: https://www.overleaf.com/learn/latex/Articles/An_Introduction_to_LuaTeX_%28Part_2%29%3A_Understanding_%5Cdirectlua

Demonstrates how to use \directlua and the LuaTeX API function texio.write to output strings directly to the .log file, useful for debugging or when direct console output is not feasible.

```tex
\directlua{
   local x="I will use \string\newcommand"
   texio.write(x)
}

```

--------------------------------

### LaTeX Mark Command Example

Source: https://www.overleaf.com/learn/latex/Articles/How_does_LaTeX_typeset_headers_and_footers%3F

This command is used internally by LaTeX to store data relevant to the current document section, which is then used for generating headers and footers.

```latex
\mark{{left}{right}}
```

--------------------------------

### Configure Headers and Footers for Odd and Even Pages

Source: https://www.overleaf.com/learn/latex/Headers_and_footers

This example configures headers and footers for both odd and even pages. By omitting the zone (L, C, or R), the specified content is applied to all three zones (left, center, and right) on the respective page types.

```latex
\begin{document}
% Set the page style to "fancy"...
\pagestyle{fancy}
%... then configure it.

% Clear all headers and footers (see also \fancyhf{})
\fancyhead{}
\fancyfoot{}

% Set the header and footer for Even
% pages but omit the zone (L, C or R)
\fancyhead[E]{Header: even page \thepage}
\fancyfoot[E]{Footer: even page \thepage}

% Set the header and footer for Odd
% pages but omit the zone (L, C or R)
\fancyhead[O]{Header: odd page \thepage}
\fancyfoot[O]{Footer: odd page \thepage}

% Some content:
This is page 1.\newpage
This is page 2.


```

--------------------------------

### Corrected array environment

Source: https://www.overleaf.com/learn/latex/Errors/Extra_alignment_tab_has_been_changed_to_%5Ccr

This example shows the corrected version of the \begin{array} environment, where the 'Extra alignment tab' error is fixed by restructuring the rows to match the defined number of columns.

```latex
\[
\begin{array}{lcl}
g(x) & = & (x+2)^2 \ 
& = & (x+2)(x+2) \ 
& = & x^2+4x+4\
\end{array}
\]
```

--------------------------------

### LaTeX document with flexible 
leftskip and 
rightskip

Source: https://www.overleaf.com/learn/latex/Articles/How_to_change_paragraph_spacing_in_LaTeX

This example demonstrates how to set flexible 
leftskip and 
rightskip values within a full LaTeX document to center paragraphs. It includes necessary packages and document structure.

```latex
\documentclass{article}
\title{leftskip and rightskip examples}
% Choose a conveniently small page size
\usepackage[paperheight==16cm,paperwidth==12cm,textwidth=8cm]{geometry}
% Set the value of some paragraph-related parameters
\setlength{\lineskip}{3.5pt}
\setlength{\lineskiplimit}{2pt}
\setlength{\parindent}{20pt}
\setlength{\baselineskip}{12pt}

\begin{document}
\input text.tex % A generated TeX file which defines the macros
% \mytextA and \mytextB which define text for two paragraphs
% Typeset a paragraph with default values of \leftskip and \rightskip

\mytextB

% Set \leftskip and \rightskip to "extremely flexible" glue settings
\leftskip 0pt plus 1fill
\rightskip 0pt plus 1fill
\vspace{20pt}

% Now re-typeset the same paragraph
\mytextB

\end{document}
```

--------------------------------

### pdfTeX Command Line Options Summary

Source: https://www.overleaf.com/learn/latex/TeX_engine_command_line_options_for_pdfTeX%2C_XeTeX_and_LuaTeX

Run this command to view a summary of pdfTeX's command line options. This provides an overview of available flags and their functions.

```bash
pdftex --help
```

--------------------------------

### Include Image in LaTeX

Source: https://www.overleaf.com/learn/latex/Positioning_of_Figures

A basic example of including an image in a LaTeX document. The default alignment is typically left.

```latex
Lorem ipsum dolor sit amet, consectetuer adipiscing elit. 
Etiam lobortis facilisis sem. Nullam nec mi et neque pharetra
sollicitudin.

\includegraphics[width=0.5\textwidth]{overleaf-logo}

Praesent imperdiet mi nec
ante. Donec ullamcorper, felis non sodales commodo, lectus velit
ultrices augue, a dignissim nibh lectus placerat pede.
 Vivamus nunc nunc, molestie ut, ultricies
vel, semper in, velit. Ut porttitor.

```

--------------------------------

### Compile to DVI with LaTeXmk

Source: https://www.overleaf.com/learn/latex/Choosing_a_LaTeX_Compiler%23Other_compilers

Use this variation of the `latexmk` command to compile your document to a DVI file, useful when PDF is not the desired output.

```bash
latexmk -dvi mydocument.tex
```

--------------------------------

### Basic Hyperlink Setup in LaTeX

Source: https://www.overleaf.com/learn/latex/Hyperlinks

Import the hyperref package in your LaTeX document's preamble to enable automatic hyperlinking for all cross-referenced elements, including table of contents entries.

```latex
\documentclass{book}
\usepackage{blindtext}
\usepackage{hyperref}

\title{Example of Hyperlinks}
\author{Overleaf}

\begin{document}

\frontmatter
\tableofcontents
\clearpage

\addcontentsline{toc}{chapter}{Foreword}
{\huge {\bf Foreword}}

\Blindtext
\clearpage

\addcontentsline{toc}{chapter}{Dummy entry}
{\huge {\bf Dummy entry}}

\Blindtext
\mainmatter

\chapter{First Chapter}

This will be an empty chapter

\begin{equation}
\label{eq:1}
\sum_{i=0}^{\infty} a_i x^i
\end{equation}

The equation \ref{eq:1} shows a sum that is divergent. This formula will be used later on page \pageref{second}.

\Blindtext
\clearpage

\section{Second section} \label{second} 

\blindtext
\Blinddocument
\end{document}
```

--------------------------------

### LuaTeX Command Token Calculation Example: \mynewmacro

Source: https://www.overleaf.com/learn/latex/Articles/An_Introduction_to_LuaTeX_%28Part_2%29%3A_Understanding_%5Cdirectlua

Shows the calculation of a command token for a user-defined macro '\\mynewmacro'. It provides the intermediate `curcs` value and the final token value.

```lua
for the user-defined macro `\mynewmacro` (a control word) LuaTeX calculates curcs=2971, resulting in a token value for `\mynewmacro` of 2971+229−1=536873882.
```

--------------------------------

### Basic Nested Fractions in LaTeX

Source: https://www.overleaf.com/learn/latex/Fractions_and_Binomials

Demonstrates how to create nested fractions. For improved typesetting of denominators, consider using \\displaystyle.

```latex
\documentclass{article}
% Using the geometry package to reduce
% the width of help article graphics
\usepackage[textwidth=9.5cm]{geometry}
% Load amsmath to access the \\cfrac{...}{...} command
\usepackage{amsmath}
\begin{document}
Fractions can be nested but, in this example, note how the default math styles, as used in the denominator, don't produce ideal results...

\[ \frac{1+\frac{a}{b}}{1+\frac{1}{1+\frac{1}{a}}} \]

\noindent ...so we use \verb|\\displaystyle| to improve typesetting:

\[ \frac{1+\frac{a}{b}} {\\displaystyle 1+\frac{1}{1+\frac{1}{a}}} \]

Here is an example which uses the \texttt{amsmath} \verb|\\cfrac| command:

\[
  a_0+\\cfrac{1}{a_1+\\cfrac{1}{a_2+\\cfrac{1}{a_3+\cdots}}}
\]

Here is another example, derived from the \texttt{amsmath} documentation, which demonstrates left
and right placement of the numerator using \verb|\\cfrac[l]| and \verb|\\cfrac[r]| respectively:
\[
\\cfrac[l]{1}{\\sqrt{2}+
\\cfrac[r]{1}{\\sqrt{2}+
\\cfrac{1}{\\sqrt{2}+\\dotsb}}}
\]
\end{document}
```

--------------------------------

### Customizing Column Separation

Source: https://www.overleaf.com/learn/latex/Multiple_columns%23Inserting_vertical_rulers

Adjust the space between columns by setting the `\columnsep` length. This example sets the separation to 1cm.

```latex
\documentclass{article}
\usepackage{blindtext}
\usepackage{multicol}
\setlength{\columnsep}{1cm}
\title{Second multicols Demo}
\author{Overleaf}
\date{April 2021}

\begin{document}
\maketitle

\begin{multicols}{2}
[
\section{First Section}
All human things are subject to decay. And when fate summons, Monarchs must obey.
]
\blindtext\blindtext
\end{multicols}

\end{document}
```

--------------------------------

### Drawing Lines and Curves with \qbezier

Source: https://www.overleaf.com/learn/latex/Picture_environment

Shows how to draw axes, lines, and quadratic Bezier curves using \line, \vector, and \qbezier commands. \thinlines and \thicklines control line thickness.

```latex
\documentclass{article}
\usepackage[pdftex]{pict2e}
\begin{document}
\setlength{\unitlength}{1cm}
\begin{picture}(8,4)
  \thinlines % Start with thin lines
  \put(0,0){\vector(1,0){8}}  % x axis
  \put(0,0){\vector(0,1){4}}  % y axis
  \put(2,0){\line(0,1){3}}    % left side
  \put(4,0){\line(0,1){3.5}}  % right side
  \thicklines % Use thicker lines for the \qbezier commands
  \qbezier(2,3)(2.5,2.9)(3,3.25)
  \qbezier(3,3.25)(3.5,3.6)(4,3.5)
  \thinlines % Back to using thin lines
  \put(2,3){\line(4,1){2}}
  \put(4.5,2.5){\framebox{Trapezoidal Rule}}
\end{picture}
\end{document}
```

--------------------------------

### Basic Arabic Document Setup

Source: https://www.overleaf.com/learn/latex/Multilingual_typesetting_on_Overleaf_using_polyglossia_and_fontspec

Set the default language to Arabic and specify a main font for basic RTL typesetting. Ensure correct rendering of LTR text within RTL context.

```latex
\usepackage{polyglossia}
\setdefaultlanguage{arabic}
\setmainfont{Amiri}
\begin{document}
ما هو differentiation
\end{document}
```

--------------------------------

### Specify Image Folder Path (Absolute)

Source: https://www.overleaf.com/learn/latex/Inserting_Images%23Captioning%2C_labelling_and_referencing

Use absolute paths to specify the exact location of image folders on your system. This is useful for local installations.

```latex
%Path in Windows format:
\graphicspath{ {c:/user/images/} }
```

```latex
%Path in Unix-like (Linux, Mac OS) format
\graphicspath{ {/home/user/images/} }
```

--------------------------------

### Display Table of Contents at Section Start

Source: https://www.overleaf.com/learn/latex/Beamer

Place this code in the preamble to automatically generate a table of contents at the beginning of each section. Use \tableofcontents[currentsection] to highlight the current section.

```latex
\AtBeginSection[]
{
  \begin{frame}
    \frametitle{Table of Contents}
    \tableofcontents[currentsection]
  \end{frame}
}
```

--------------------------------

### Basic AVM Structure with 'avm' package

Source: https://www.overleaf.com/learn/latex/Attribute_Value_Matrices

Demonstrates the fundamental structure of an Attribute Value Matrix using the 'avm' environment. Ensure the 'avm.sty' file is available in your project.

```latex
\documentclass{article}
\usepackage{avm}

\begin{document}

\begin{avm}
    [ cat|subcat & <NP$_{it}$, NP$_{\@2}$, S[comp]:\@3> \\
       content & [ relation & \bf bother\\
                    bothered & \@2 \\
                    soa-arg  & \@3 ] ]
\end{avm}

\end{document}

```

--------------------------------

### Pre-processed Lua Code Example

Source: https://www.overleaf.com/learn/latex/Articles/An_Introduction_to_LuaTeX_%28Part_2%29%3A_Understanding_%5Cdirectlua

Illustrates the pre-processed code passed to Lua when using \directlua with macros. Unprotected macros are expanded, and protected macros remain as literal tokens.

```lua
local x="This unprotected macro contains a string"..\macroB 
```

--------------------------------

### Complete LaTeX Document for Greek Text

Source: https://www.overleaf.com/learn/latex/Greek

This is a full LaTeX document example demonstrating how to typeset Greek text. It includes necessary packages for Greek language support and output encoding. Use this as a template for your Greek documents.

```latex
\documentclass{article}
% \usepackage[utf8]{inputenc} is no longer required (since 2018)

% Set the font (output) encoding
\usepackage[LGR]{fontenc}

% Greek-specific commands
\usepackage[greek]{babel}

\begin{document}
\tableofcontents

\begin{abstract}
Αυτή είναι μια σύντομη περιγραφή του θέματος 
sαφέστερα εξηγείται στο παρόν έγγραφο
\end{abstract}

\section{εισαγωγή}
Αυτό είναι το πρώτο τμήμα του εγγράφου. 
Είναι μια εισαγωγική παράγραφος.

\section{δεύτερο τμήμα}
Το δεύτερο τμήμα του εγγράφου. Αυτή η ενότητα 
μπορεί να περιέχει μαθηματική σημειογραφία.
\end{document}

```

--------------------------------

### Draw a Simple Curve

Source: https://www.overleaf.com/learn/latex/Articles/How_to_draw_Vector_Graphics_using_TikZ_in_LaTeX

Create a curve by specifying the start and end points, along with the outgoing and incoming angles.

```latex
\begin{tikzpicture}
\draw[ultra thick]
(0,0) to [out=75,in=135](3,4);
\end{tikzpicture}
```

--------------------------------

### Basic 3-Column Layout with multicol

Source: https://www.overleaf.com/learn/latex/Multiple_columns

Use the \begin{multicols}{N} environment to create N columns. An optional header text can be provided in square brackets.

```latex
\documentclass{article}
\usepackage{blindtext}
\usepackage{multicol}
\title{Multicols Demo}
\author{Overleaf}
\date{April 2021}

\begin{document}
\maketitle

\begin{multicols}{3}
[
\section{First Section}
All human things are subject to decay. And when fate summons, Monarchs must obey.
]
\blindtext\blindtext
\end{multicols}

\end{document}

```

--------------------------------

### Basic Text Formatting in LaTeX

Source: https://www.overleaf.com/learn/latex/Creating_a_document_in_LaTeX

Shows how to apply bold, italics, and underlining to text using \textbf, \textit, and \underline commands respectively.

```latex
Some of the \textbf{greatest}
discoveries in \underline{science} 
were made by \textbf{\textit{accident}}.


```

--------------------------------

### Markdown recipe structure

Source: https://www.overleaf.com/learn/latex/Articles/Markdown_into_LaTeX_with_Style

Example of a recipe structured using Markdown, including headings, key-value pairs for metadata, and lists for ingredients and instructions. This format can be styled using LaTeX.

```markdown
# Scrambled Eggs

Cooking time
: 5 minutes

Serves
: 1

## Ingredients
- 4 eggs
- 1 tsp butter
- 1/4 cup milk
- salt, pepper (to taste)

## Instructions

#. Beat eggs, milk...
#. Heat butter in pan...
#. ...

```

--------------------------------

### Advanced Powerdot Presentation with Options

Source: https://www.overleaf.com/learn/latex/Powerdot

Configure Powerdot presentations with options like print mode, paper size, and orientation. This example demonstrates setting metadata and creating multiple slides with different content.

```latex
\documentclass[
    mode=print,
    paper=smartboard,
    orient=landscape
]{powerdot}

% Presentation metadata
\title{Powerdot Presentation}
\author{Overleaf}
\date{\today}

\begin{document}
\maketitle
     
% section: title takes up full slide
\section{First section}
           
\begin{slide}{Slide Title}
    \begin{itemize}
    \item This is an item
    \item Second item
    \item Third item
    \end{itemize}
\end{slide}
                                         
\begin{slide}{Slide N 2}
    This is the content of slide 2.
    Math $x=2\pi r$.
\end{slide}
\end{document}

```

--------------------------------

### Get the current value of 
\baselineskip

Source: https://www.overleaf.com/learn/latex/Articles/How_to_change_paragraph_spacing_in_LaTeX

Use \the\baselineskip to retrieve and display the current preferred interline spacing value in TeX/LaTeX.

```latex
\the\baselineskip
```

--------------------------------

### Configure Biblatex and Add Bibliography

Source: https://www.overleaf.com/learn/latex/Biblatex_bibliography_styles

Use these commands in the preamble to set the backend, bibliography style, and add your .bib file. Ensure 'biber' is used as the backend for full functionality.

```latex
%in the preamble
%--------------------------------
  \usepackage[
    backend=biber,
    style=stylename,
  ]{biblatex}

 \addbibresource{bibfile}
%--------------------------------

%Where the bibliography will be printed
  \printbibliography

```

--------------------------------

### Testing for Math Mode with \ifmmode

Source: https://www.overleaf.com/learn/latex/Errors/Missing_%24_inserted

This example demonstrates how to use the \ifmmode command to create a macro that behaves differently depending on whether TeX is currently in math mode. It shows usage within inline math ($...$), display math ($$...$$), LaTeX inline math (\(...\)), and display math (\[...\]), as well as outside of any math mode.

```latex
\documentclass{article}
\begin{document}
\newcommand{\mytest}{\ifmmode \mathrm{Yes}\else No\fi.}

Is the macro being used in math mode? $\mytest$

Is the macro being used in math mode? $$\mytest$$

Is the macro being used in math mode? \(\mytest

Is the macro being used in math mode? \[\mytest\]

Is the macro being used in math mode? \mytest
\end{document}
```

--------------------------------

### Define a custom LaTeX package

Source: https://www.overleaf.com/learn/latex/Management_in_a_large_project%23Inputting_and_including_files

Create a .sty file to organize preamble code, such as user-defined commands or glossary terms. This prevents the main file from becoming too long and confusing. The file must start with \ProvidesPackage{packagename}.

```latex
\ProvidesPackage{example}

\usepackage{amsmath}
\usepackage{amsfonts}
\usepackage{amssymb}
\usepackage[latin1]{inputenc}
\usepackage[spanish, english]{babel}
\usepackage{graphicx}
\usepackage{blindtext}
\usepackage{textcomp}
\usepackage{pgfplots}

\pgfplotsset{width=10cm,compat=1.9}

%Header styles
\usepackage{fancyhdr}
\setlength{\headheight}{15pt}
\pagestyle{fancy}
\renewcommand{\chaptermark}[1]{\markboth{#1}{}}
\renewcommand{\sectionmark}[1]{\markright{#1}{}}
\fancyhf{}
\fancyhead[LE,RO]{\thepage}
\fancyhead[RE]{\textbf{\textit{\nouppercase{\leftmark}}}} 
\fancyhead[LO]{\textbf{\textit{\nouppercase{\rightmark}}}}
\fancypagestyle{plain}{ % 
\fancyhf{} % remove everything
\renewcommand{\headrulewidth}{0pt} % remove lines as well
\renewcommand{\footrulewidth}{0pt}}

%makes available the commands \proof, \qedsymbol and \theoremstyle
\usepackage{amsthm}

%Ruler
\newcommand{\HRule}{\rule{\linewidth}{0.5mm}}

%Lemma definition and lemma counter
\newtheorem{lemma}{Lemma}[section]

%Definition counter
\theoremstyle{definition}
\newtheorem{definition}{Definition}[section]

%Corolary counter
\newtheorem{corolary}{Corolary}[section]

%Commands for naturals, integers, topology, hull, Ball, Disc, Dimension, boundary and a few more
\newcommand{\E}{{\mathcal{E}}}
\newcommand{\F}{{\mathcal{F}}}
...

%Example environment
\theoremstyle{remark}
\newtheorem{examle}{Example}

%Example counter
\newcommand{\reiniciar}{\setcounter{example}{0}}

```

--------------------------------

### Default Sorting Order of Nomenclatures

Source: https://www.overleaf.com/learn/latex/Nomenclatures

This example demonstrates the default sorting behavior of the nomencl package without any manual sorting prefixes.

```latex
\documentclass{article}
\usepackage{nomencl}

\makenomenclature

\begin{document}
Here is an example:

\nomenclature{\(+a\)}{Operator}
\nomenclature{\(2a\)}{Number}
\nomenclature{\:a\)}{Punctuation symbol}
\nomenclature{\(Aa\)}{Uppercase letter}
\nomenclature{\(aa\)}{Lowercase letter}
\nomenclature{\(\alpha\)}{Greek character}

\printnomenclature

\end{document}
```

--------------------------------

### Example LaTeX Document with BibTeX

Source: https://www.overleaf.com/learn/latex/Bibliography_styles%23Natbib_styles

A complete LaTeX document demonstrating BibTeX usage, including package inclusions, document structure, citations, and setting the bibliography style to 'unsrt' with the 'sample.bib' file.

```latex
\documentclass[a4paper,10pt]{article}
\usepackage[english]{babel}
%Includes "References" in the table of contents
\usepackage[nottoc]{tocbibind}

%Title, date an author of the document
\title{Bibliography management: BibTeX}
\author{Overleaf}

%Begining of the document
\begin{document}

\maketitle

\tableofcontents

\medskip

\section{First Section}
This document is an example of BibTeX using in bibliography management. Three items are cited: \textit{The \LaTeX\ Companion} book \cite{latexcompanion}, the Einstein journal paper \cite{einstein}, and the Donald Knuth's website \cite{knuthwebsite}. The \LaTeX\ related items are \cite{latexcompanion,knuthwebsite}. 

\medskip

%Sets the bibliography style to UNSRT and imports the 
%bibliography file "sample.bib".
\bibliographystyle{unsrt}
\bibliography{sample}
\end{document}
```

--------------------------------

### Fancyhdr Warning Messages

Source: https://www.overleaf.com/learn/latex/Headers_and_footers

Example of warning messages generated by the fancyhdr package when 'even' page coordinates are used in single-sided documents.

```text
Package fancyhdr Warning: \fancyhead's `E' option without twoside option is useless on input line 23.
Package fancyhdr Warning: \fancyfoot's `E' option without twoside option is useless on input line 24.
```

--------------------------------

### Drawing a Bézier Curve with qbezier

Source: https://www.overleaf.com/learn/latex/Picture_environment

Draws a Bézier curve using the \qbezier command. The command takes start, control, and end points. It is not typically used within a \put command.

```latex
\documentclass{article}
\usepackage[pdftex]{pict2e}
\begin{document}
\setlength{\unitlength}{0.8cm}
\begin{picture}(10,5)
\thicklines
\qbezier(1,1)(5,5)(9,0.5)
\put(2,1){{Bézier curve}}
\end{picture}
\end{document}
```

--------------------------------

### Enable SyncTeX using TeX primitive

Source: https://www.overleaf.com/learn/latex/MLTeX_SyncTeX_and_EncTeX_TeX_extensions

Add \synctex=1 to your .tex file to enable SyncTeX.

```tex
\synctex=1

```

--------------------------------

### LaTeX Counter Error Example

Source: https://www.overleaf.com/learn/latex/Counters

Illustrates an error that occurs when \value is used incorrectly as a command to output a counter's value.

```latex
%\value{first}
```

--------------------------------

### LaTeX document with grouped 
leftskip and 
rightskip changes

Source: https://www.overleaf.com/learn/latex/Articles/How_to_change_paragraph_spacing_in_LaTeX

This example demonstrates applying flexible 
leftskip and 
rightskip settings temporarily within a group in a LaTeX document. It shows how text inside the group is affected, while text outside reverts to default settings.

```latex
\documentclass{article}
\title{Another leftskip and rightskip example}
% Choose a conveniently small page size
\usepackage[paperheight=16cm,paperwidth=12cm,textwidth=8cm]{geometry}
% Set the value of some paragraph-related parameters
\setlength{\lineskip}{3.5pt}
\setlength{\lineskiplimit}{2pt}
\setlength{\parindent}{20pt}
\setlength{\baselineskip}{12pt}

\begin{document}
\input text.tex % A generated TeX file which defines the macros
% \mytextA and \mytextB which define text for two paragraphs
% Here we change \leftskip and \rightskip to "extremely flexible" glue settings
% within a group created by \begingroup ... \endgroup

\begingroup
\leftskip 0pt plus 1fill
\rightskip 0pt plus 1fill
\mytextB
\endgroup

% Now re-typeset the same paragraph after the group has closed
% causing \leftskip and \rightskip revert to their previous values 

\vspace{20pt}
\mytextB

\end{document}
```

--------------------------------

### Use \directlua with % Character and Lua String Functions

Source: https://www.overleaf.com/learn/latex/Articles/An_Introduction_to_LuaTeX_%28Part_2%29%3A_Understanding_%5Cdirectlua

An extensive example demonstrating the use of \directlua with the % character, including changing its category code to 12 and utilizing Lua's string.format, string.gmatch, and string.gsub functions.

```tex
\documentclass{article}
\begin{document}
\begingroup
\ttfamily
\let\
elax
\catcode`\^^M=12 
\catcode`\%=12
\directlua{
   local str -- declare a local variable to hold the result

   tex.print("Using string.format():".."\\par")

   str=string.format("%s %q", "Hello", "Lua user!") -- string and quoted string
   tex.print(str.."\\par")
   str = string.format("%c%c%c", 76, 117, 97) -- char
   tex.print(str.."\\par")
   str=string.format("%e, %E", math.pi, math.pi) -- exponent
   tex.print(str.."\\par")
   str=string.format("%f", math.pi) -- float
   tex.print(str.."\\par")
   str=string.format("%g, %g", math.pi, 10^9) -- float or exponent
   tex.print(str.."\\par")
   str = string.format("%o, %x, %X", 99, 125, 125)  -- octal, hexadecimal, hexadecimal
   tex.print(str.."\\par")

   tex.print("\\vskip3mm".."Using string.gmatch():".."\\par")

   for word in string.gmatch("Hello TeX user", "%a+") do 
      tex.print(word.."\\par")
   end

   tex.print("\\vskip3mm".."Using string.gsub():".."\\par")
   str=string.gsub("banana", "(an)", "%1-") -- capture any occurrences of "an" and replace
   tex.print(str.."\\par")
}
\endgroup
\end{document}

```

--------------------------------

### Token List for 'Hello'

Source: https://www.overleaf.com/learn/latex/Articles/What_is_a_%22TeX_token%22%3F

Illustrates the internal representation of the string 'Hello' as a list of TeX tokens, showing the calculation for each character based on its category code and ASCII value.

```tex
H→ 256 × 11 + 72 = 2888
```

```tex
e→ 256 × 11 + 101 = 2917
```

```tex
l→ 256 × 11 +108 = 2924
```

```tex
l→ 256 × 11 +108 = 2924
```

```tex
o→256 × 11 + 111 = 2927
```

--------------------------------

### Command Token Calculation Example

Source: https://www.overleaf.com/learn/latex/Articles/How_does_%5Cexpandafter_work%3A_An_introduction_to_TeX_tokens

Illustrates the calculation of a command token for commands constructed from characters with category code 11. It involves calculating a 'curcs' value and adding 4095.

```tex
command token = curcs + 4095
Note that the variable `curcs` plays an extremely important role in TeX’s inner processing activities.
```

--------------------------------

### BibTeX Bibliography File Example

Source: https://www.overleaf.com/learn/latex/Bibliography_management_with_bibtex

A sample \texttt{.bib} file containing various reference entry types like \texttt{@book} and \texttt{@article}. This file stores citation information in a format-independent way.

```bibtex
@book{texbook,
  author = {Donald E. Knuth},
  year = {1986},
  title = {The {\TeX} Book},
  publisher = {Addison-Wesley Professional}
}

@book{latex:companion,
  author = {Frank Mittelbach and Michel Gossens
            and Johannes Braams and David Carlisle
            and Chris Rowley},
  year = {2004},
  title = {The {\LaTeX} Companion},
  publisher = {Addison-Wesley Professional},
  edition = {2}
}

@book{latex2e,
  author = {Leslie Lamport},
  year = {1994},
  title = {{\LaTeX}: a Document Preparation System},
  publisher = {Addison Wesley},
  address = {Massachusetts},
  edition = {2}
}

@article{knuth:1984,
  title={Literate Programming},
  author={Donald E. Knuth},
  journal={The Computer Journal},
  volume={27},
  number={2},
  pages={97--111},
  year={1984},
  publisher={Oxford University Press}
}

@inproceedings{lesk:1977,
  title={Computer Typesetting of Technical Journals on {UNIX}},
  author={Michael Lesk and Brian Kernighan},
  booktitle={Proceedings of American Federation of
             Information Processing Societies: 1977
             National Computer Conference},
  pages={879--888},
  year={1977},
  address={Dallas, Texas}
}

```

--------------------------------

### LaTeX Prescript Example

Source: https://www.overleaf.com/learn/latex/Articles/Mathtools_-_for_beautiful_math

Use \prescript to add superscripts and subscripts before a symbol, commonly used in chemistry for atomic numbers and masses.

```latex
\prescript{238}{92}{\mathbf{U}}
```

--------------------------------

### LaTeX Table with booktabs Package

Source: https://www.overleaf.com/learn/how-to/How_to_insert_tables_in_Overleaf%23editable

This example demonstrates using the booktabs package for enhanced horizontal rules in LaTeX tables. It includes commands like \toprule, \midrule, and \bottomrule for professional table formatting.

```latex
\documentclass{article}
\usepackage{booktabs}
\usepackage{hologo} % for the XeTeX logo
\begin{document}
\begin{table}
\begin{tabular}{lcc}
\toprule 
    \TeX{} engine&Year released&Native UTF-8\\
\midrule 
    pdf\TeX&1996&No\\
    \hologo{XeTeX}&2004&Yes\\
    Lua\TeX&2007&Yes\\
    LuaHB\TeX&2019&Yes\\
\bottomrule
\end{tabular}
\end{table}
\end{document}
```

--------------------------------

### Single-sided Document Headers and Footers

Source: https://www.overleaf.com/learn/latex/Headers_and_footers

Configure headers and footers for single-sided documents. This example demonstrates setting headers and footers without the 'even' page coordinates, which are not applicable.

```latex
\documentclass{article}
\usepackage[paperheight=16cm, paperwidth=12cm,% Set the height and width of the paper
includehead,
nomarginpar,% We don't want any margin paragraphs
textwidth=10cm,% Set \textwidth to 10cm
headheight=10mm,% Set \headheight to 10mm
]{geometry}
\usepackage{fancyhdr}
\begin{document}
% Set the page style to "fancy"...
\pagestyle{fancy}
\title{Single-sided document}
\author{Overleaf}
\date{August 2022}
\fancyhf{} % clear existing header/footer entries
% We don't need to specify the O coordinate
\fancyhead[R]{Hello}
\fancyfoot[L]{\thepage}
\maketitle
\section{Introduction}
Some content.
\newpage
\section{Continued...}
\end{document}
```

--------------------------------

### Macro Definition for \foo

Source: https://www.overleaf.com/learn/latex/Articles/How_does_%5Cexpandafter_work%3A_A_detailed_study_of_consecutive_%5Cexpandafter_commands

Defines the \foo macro to make text bold. This is a prerequisite for understanding the \expandafter examples that use \foo.

```latex
\def\foo#1{\textbf{#1}}
```

--------------------------------

### Demonstrate \parskip values in LaTeX

Source: https://www.overleaf.com/learn/latex/Articles/How_to_change_paragraph_spacing_in_LaTeX

This example showcases how different values of \parskip affect paragraph spacing, including positive, negative, and grouped settings. It uses geometry for page size and defines a command to display text and the current \parskip value.

```latex
\documentclass{article}
% Choose a conveniently small page size
\usepackage[paperheight=16cm,paperwidth=12cm,textwidth=8cm]{geometry}
% Create a command to hold a paragraph of text
\newcommand{\testpar}{\noindent The current value of \texttt{\string\parskip} is \texttt{\the\parskip}. When \TeX{} typesets a paragraph it treats each individual character as a 2D~``box'' with a specific width, height and depth.\par}
\begin{document}
\testpar
% Set \parskip to 5pt 
\setlength{\parskip}{5pt}\testpar
% Set parskip to 10pt
\setlength{\parskip}{10pt}\testpar
% Set \parskip to 25pt (but in a group)
{\setlength{\parskip}{25pt}\testpar}
% Group ends, now \parskip is again 10pt
\testpar
% Yes, you can have a negative \parskip
\setlength{\parskip}{-20pt}\testpar
\end{document}
```

--------------------------------

### Draw Complex Curves

Source: https://www.overleaf.com/learn/latex/Articles/How_to_draw_Vector_Graphics_using_TikZ_in_LaTeX

Construct intricate shapes by chaining 'to' commands with specified angles to guide the curve's path.

```latex
\begin{tikzpicture}
\draw[ultra thick] 
(0,0) 
to [out=90,in=180] (2,2)
 to [out=270,in=0](0,0)
 to [out=0,in=90](2,-2)
 to [out=180,in=270](0,0)
 to [out=270,in=0](-2,-2)
 to [out=90,in=180](0,0)
 to [out=180,in=270](-2,2)
 to [out=0,in=90](0,0);
\end{tikzpicture}
```

--------------------------------

### Adding Entries and Sub-entries to LaTeX Index

Source: https://www.overleaf.com/learn/latex/Indices

This example shows how to create nested entries (sub-entries) in a LaTeX index. Sub-entries are created by using an exclamation mark '!' within the \index command.

```latex
\documentclass{article}
\usepackage[T1]{fontenc}
\usepackage{imakeidx}
\makeindex

\begin{document}
\section{Introduction}
In this example, several keywords\index{keywords} will be used 
which are important and deserve to appear in the Index\index{Index}.

Terms like generate\index{generate} and some\index{others} will also 
show up. Terms in the index can also be nested \index{Index!nested}

\clearpage

\section{Second section}
This second section\index{section} may include some special word, 
and expand the ones already used\index{keywords!used}.

\printindex
\end{document}

```

--------------------------------

### Define Custom LaTeX Command with Parameters

Source: https://www.overleaf.com/learn/latex/Commands%23Defining_a_new_command

Demonstrates defining a new LaTeX command \bb that accepts one parameter to represent numerical systems (like complex, rational, integer) in blackboard boldface.

```latex
\documentclass{article}
\usepackage{amssymb}
\begin{document}
\newcommand{\bb}[1]{\mathbb{#1}}
Other numerical systems have similar notations. 
The complex numbers \( \bb{C} \), the rational 
numbers \( \bb{Q} \) and the integer numbers \( \bb{Z} \).
\end{document}


```

--------------------------------

### Setting Chinese Main Font

Source: https://www.overleaf.com/learn/latex/Chinese

Use `\setCJKmainfont{}` to specify a font for Chinese characters, such as `BabelStone Han`, if it's installed on your system.

```latex
\setCJKmainfont{BabelStone Han}
```

--------------------------------

### Fenced Code Block Example in Markdown

Source: https://www.overleaf.com/learn/latex/Articles/Markdown_into_LaTeX_with_Style

Demonstrates the use of tildes as fences for code blocks in markdown. Ensure updated markdown package files are used for this feature.

```markdown
~~~~ 
int main() {
  cout << "Hello world!";
  return 0;
}

~~~~
```

--------------------------------

### Loading the dirtytalk Package

Source: https://www.overleaf.com/learn/latex/Typesetting_quotations

Shows how to load the `dirtytalk` package in the document preamble.

```latex
\usepackage{dirtytalk}
```

--------------------------------

### LuaLaTeX document with ltjarticle class

Source: https://www.overleaf.com/learn/latex/Japanese%23The_pTeX_engine

Example of a Japanese document using the `ltjarticle` document class with LuaLaTeX, leveraging the luatex-ja package bundle.

```latex
\documentclass{ltjarticle}
\begin{document}
\section{これは最初のセクションである}
日本語で \LaTeX の組版を実証するための導入部分。

フォントはまた、数学的な形態および他の環境で使用することができる
\end{document}
```

--------------------------------

### Basic LaTeX Document Structure with Commands

Source: https://www.overleaf.com/learn/latex/Commands

Demonstrates a minimal LaTeX document structure and the use of \\textbf for bold text and mathematical symbols.

```latex
\documentclass{article}
\begin{document}
In a document there are different types of \textbf{commands} 
that define the way the elements are displayed. This 
commands may insert special elements: $\\alpha \\beta \\Gamma$
\end{document}


```

--------------------------------

### LaTeX Error Message Example

Source: https://www.overleaf.com/learn/latex/Errors/LaTeX_Error%3A_Missing_%5Cbegin_document

This is the error message generated by LaTeX when \begin{document} is missing. Ensure \begin{document} is present after the preamble.

```latex
LaTeX Error: Missing \begin{document}.
See the LaTeX manual or LaTeX Companion for explanation.
Type H <return> for immediate help.
... You're in trouble here. Try typing <return> to proceed. If that doesn't work, type X <return> to quit.
```

--------------------------------

### Using the `spacing` Environment

Source: https://www.overleaf.com/learn/latex/Paragraph_formatting

Apply custom line spacing to a specific section of your document using the `spacing` environment with a desired `\baselinestretch` value. The example demonstrates applying different spacing values and reverting to single spacing.

```latex
\documentclass{article}
% Choose a conveniently small page size
\usepackage[paperheight=18cm,paperwidth=14cm,textwidth=12cm]{geometry}
% Load blindtext package for dummy text
\usepackage{blindtext}
% Load the setspace package
\usepackage{setspace}
% Using \doublespacing in the preamble 
% changes text to double line spacing
\doublespacing
\begin{document}
\blindtext[1]

Now, apply a larger (custom) spacing by applying a \verb|\baselinestretch| value of 3:

\begin{verbatim}
\begin{spacing}{3}
\blindtext[1]
\end{spacing}
\end{verbatim}

\begin{spacing}{2.5}
\blindtext[1]
\end{spacing}
Now, revert to \verb|\singlespacing|\singlespacing\blindtext[1]
\end{document}
```

--------------------------------

### Corrected Display Math Termination

Source: https://www.overleaf.com/learn/latex/Errors/Display_math_should_end_with_%24%24

This example shows the correct way to terminate display math by ensuring it is enclosed with double dollar signs ($$).

```latex
\documentclass{article}
\begin{document}
\noindent The solution is to ensure correct termination of the 
display math by writing \verb|$$E=mc^2$$|:

$$E=mc^2$$
\end{document}

```

--------------------------------

### Multilingual Calendar Setup

Source: https://www.overleaf.com/learn/latex/Articles/How_to_create_a_multilingual%2C_customisable_CD_disk_jewel_case_calendar_using_LaTeX

Enables multilingual support for dates, month names, and weekday initials by passing the desired language as an option to the `cdcalendar` class. LuaLaTeX is recommended for `french` language option.

```latex
\documentclass[12pt,spanish]{cdcalendar}
```

--------------------------------

### Missing Both $$ Terminators

Source: https://www.overleaf.com/learn/latex/Errors/Display_math_should_end_with_%24%24

This example omits both closing dollar signs for display math, triggering 'Missing $ inserted' and 'Display math should end with $$' errors.

```latex
\documentclass{article}
\usepackage[textwidth=8cm]{geometry}
\begin{document}
\noindent The following example omits both terminating \texttt{\$} characters, triggering the errors \texttt{Missing \$ inserted} and \texttt{Display math should end with \$\$.}

$$E=mc^2
\end{document}

```

--------------------------------

### Corrected pmatrix environment by increasing MaxMatrixCols

Source: https://www.overleaf.com/learn/latex/Errors/Extra_alignment_tab_has_been_changed_to_%5Ccr

This example demonstrates how to fix the 'Extra alignment tab' error in an \amsmath \begin{pmatrix} environment by increasing the \MaxMatrixCols counter to accommodate the desired number of columns.

```latex
\documentclass{article}
\usepackage{amsmath} % To access the pmatrix environment
\setcounter{MaxMatrixCols}{12}
\begin{document}
\[
\begin{pmatrix}
a_{1,1} & a_{1,2} & a_{1,3} & a_{1,4} & a_{1,5} & a_{1,6} & a_{1,7} & a_{1,8} & a_{1,9} & a_{1,10} & a_{1,11} & a_{1,12} \ 
\end{pmatrix}
\]
\end{document}
```

--------------------------------

### BibTeX @article Entry Example

Source: https://www.overleaf.com/learn/latex/Bibliography_management_with_bibtex

Use the @article entry type for journal articles. Ensure 'author', 'year', and 'title' fields are included.

```bibtex
@article{knuth:1984,
  title={Literate Programming},
  author={Donald E. Knuth},
  journal={The Computer Journal},
  volume={27},
  number={2},
  pages={97--111},
  year={1984},
  publisher={Oxford University Press}
}


```

--------------------------------

### Using \char with \codestoemoji

Source: https://www.overleaf.com/learn/latex/Articles/An_overview_of_technologies_supporting_the_use_of_colour_emoji_fonts_in_LaTeX

This example shows an incorrect usage of \char with \codestoemoji, which leads to a Lua syntax error because \char is not an expandable command.

```latex
\codestoemoji{\char"1F3F4\Uchar"E0067\Uchar"E0062\Uchar"E0065\Uchar"E006E\Uchar"E0067\Uchar"E007F}

```

--------------------------------

### Drawing Caffeine Structure with chemfig

Source: https://www.overleaf.com/learn/latex/Chemistry_formulae

A complex example demonstrating the drawing of connected rings and various bond types to represent the caffeine molecule.

```latex
\documentclass{article}
\usepackage{chemfig}
\begin{document}
\section{I need caffeine.}

\chemfig{*6((=O)-N(-)-(*5(-N=-N(-)-))=-(=O)-N(-)-)}
\end{document}
```

--------------------------------

### Various Display Math Environments in LaTeX

Source: https://www.overleaf.com/learn/latex/Mathematical_expressions

Demonstrates different ways to typeset mathematical expressions in display mode, including unnumbered equations using \[...\] and \begin{equation*}...
\end{equation*}, and \begin{displaymath}...
\end{displaymath}.

```latex
\documentclass{article}
\usepackage{amsmath} % for the equation* environment
\begin{document}

This is a simple math expression \(\sqrt{x^2+1}\) inside text. 
And this is also the same: 
\begin{math}
\sqrt{x^2+1}
\end{math}
but by using another command.

This is a simple math expression without numbering
\[\sqrt{x^2+1}\] 
separated from text.

This is also the same:
\begin{displaymath}
\sqrt{x^2+1}
\end{displaymath}

\ldots and this:
\begin{equation*}
\sqrt{x^2+1}
\end{equation*}

\end{document}

```

--------------------------------

### LuaTeX Interfacing Primitives for pdfTeX Compatibility

Source: https://www.overleaf.com/learn/latex/TeX_primitives_cross-reference_data

Demonstrates how to implement pdfTeX primitive functionality in LuaTeX using interfacing primitives like \pdfextension, \pdfvariable, and \pdffeedback. These examples are derived from The LuaTeX Reference Manual.

```tex
\protected\def\pdfliteral{\pdfextension literal}
\def\pdftexrevision{\pdffeedback revision}
\edef\pdfpagebox{\pdfvariable pagebox}

```

--------------------------------

### Define Theorem Styles with amsthm

Source: https://www.overleaf.com/learn/latex/Theorems_and_proofs

Use \theoremstyle to set the formatting for theorem-like environments. Define custom environments like 'definition' and 'remark' using \newtheorem.

```latex
\documentclass{article}
\usepackage[english]{babel}
\usepackage{amsthm}

\theoremstyle{definition}
\newtheorem{definition}{Definition}[section]

\theoremstyle{remark}
\newtheorem*{remark}{Remark}

\begin{document}
\section{Introduction}
Unnumbered theorem-like environments are also possible.

\begin{remark}
This statement is true, I guess.
\end{remark}

And the next is a somewhat informal definition

\begin{definition}[Fibration]
A fibration is a mapping between two topological spaces that has the homotopy lifting property for every space \(X\).
\end{definition}
\end{document}
```

--------------------------------

### Macro Definition for \bar

Source: https://www.overleaf.com/learn/latex/Articles/How_does_%5Cexpandafter_work%3A_A_detailed_study_of_consecutive_%5Cexpandafter_commands

Defines the \bar macro, which itself expands to \abc and \xyz. This macro is used in \expandafter examples to show nested expansion.

```latex
\def\abc{Hello}
```

```latex
\def\xyz{, World!}
```

```latex
\def\bar{\abc\xyz}
```

--------------------------------

### Define Numbered Environment with \newtheorem

Source: https://www.overleaf.com/learn/latex/Environments%23Defining_a_new_environment

Define a numbered environment using the \newtheorem command from the amsmath package. The environment is reset at the start of each section.

```latex
\usepackage{amsmath} % For the \newtheorem command
\newtheorem{SampleEnv}{Sample Environment}[section]
```

--------------------------------

### Defining a New Environment: General Form

Source: https://www.overleaf.com/learn/latex/Environments%23Defining_a_new_environment

Presents the general syntax for defining a new environment using the \newenvironment command in LaTeX.

```latex
\newenvironment{name}[numarg][optarg_default]{begin_def}{end_def}
```

--------------------------------

### Advanced LaTeX Table Formatting

Source: https://www.overleaf.com/learn/latex/Creating_a_document_in_LaTeX

This example demonstrates more complex table formatting with double vertical lines (`||`), varied column content, and spacing adjustments using `[0.5ex]` and `[1ex]` after row endings.

```latex
\begin{center}
 \begin{tabular}{||c c c c||} 
 \hline
 Col1 & Col2 & Col2 & Col3 \\ [0.5ex] 
 \hline\hline
 1 & 6 & 87837 & 787 \\ 
 \hline
 2 & 7 & 78 & 5415 \\
 \hline
 3 & 545 & 778 & 7507 \\
 \hline
 4 & 545 & 18744 & 7560 \\
 \hline
 5 & 88 & 788 & 6344 \\ [1ex] 
 \hline
\end{tabular}
\end{center}
```

--------------------------------

### LaTeX: Using Font Families (Commands and Switches)

Source: https://www.overleaf.com/learn/latex/Font_sizes%2C_families%2C_and_styles

Shows how to change font families using commands like \texttt and switches like \sffamily. Commands affect specific text, while switches affect text from that point onwards.

```latex
In this example, a command and a switch are used. 
\texttt{A command is used to change the style 
of a sentence}.

\sffamily
A switch changes the style from this point to 
the end of the document unless another switch is used.

```

--------------------------------

### Get current \parskip value

Source: https://www.overleaf.com/learn/latex/Articles/How_to_change_paragraph_spacing_in_LaTeX

This TeX command retrieves and typesets the current value of the \parskip parameter, which controls the vertical space between paragraphs.

```tex
\the\parskip
```

--------------------------------

### Change QED Symbol in LaTeX Proofs

Source: https://www.overleaf.com/learn/latex/Theorems_and_proofs

Customize the end-of-proof symbol using \renewcommand\qedsymbol. This example demonstrates changing it to a black square and then to the word 'QED'.

```latex
\documentclass{article}

\usepackage[english]{babel}
\usepackage{amsthm}
\usepackage{amssymb}

\newtheorem{theorem}{Theorem}[section]
\newtheorem{lemma}[theorem]{Lemma}

\begin{document}
\section{Introduction}

\begin{lemma}
Given two line segments whose lengths are \(a\) and \(b\) respectively there 
is a real number \(r\) such that \(b=ra\).
\end{lemma}

\renewcommand\qedsymbol{$\\blacksquare$}

\begin{proof}
To prove it by contradiction try and assume that the statement is false,
proceed from there and at some point you will arrive to a contradiction.
\end{proof}

\renewcommand\qedsymbol{QED}

\begin{proof}
To prove it by contradiction try and assume that the statement is false,
proceed from there and at some point you will arrive to a contradiction.
\end{proof}
\end{document}
```

--------------------------------

### Demonstrate \indent Command in LaTeX

Source: https://www.overleaf.com/learn/latex/Paragraphs_and_new_lines

This example shows how \indent affects paragraph indentation when used within text, inline math, and an \hbox. It requires the geometry package for page layout.

```latex
\documentclass{article}
% Using the geometry package with a small
% page size to create the article graphic
\usepackage[paperheight=6in,
   paperwidth=5in,
   top=10mm,
   bottom=20mm,
   left=10mm,
   right=10mm]{geometry}
\begin{document}
\noindent A new paragraph with some text, then an \verb|\indent|\indent command. Next, some inline math which also has an indent $y\indent x$. \verb|\indent| also works when used in an \verb|\hbox| such as \verb|\hbox{A\indent B}| which produces \hbox{A\indent B}.
\end{document}
```

--------------------------------

### Creating and Using HarfBuzz Font Features

Source: https://www.overleaf.com/learn/latex/Articles/An_overview_of_technologies_supporting_the_use_of_colour_emoji_fonts_in_LaTeX

Demonstrates how to create HarfBuzz font feature objects using Feature.new() and apply them during the shape_full() operation. This example activates small capitals and disables kerning.

```lua
local dosmcp = hblib.Feature.new("+smcp")
local nokern = hblib.Feature.new("-kern")
% Use your font features like this
local res = hblib.shape_full(hbfont, hbbuffer, {dosmcp,nokern},{})

```

--------------------------------

### Text-Style Fractions with amsmath

Source: https://www.overleaf.com/learn/latex/Fractions_and_Binomials

Demonstrates using the \text{...} command from the amsmath package to typeset fractions containing literal text, preventing LaTeX from interpreting the text as mathematical symbols. The example shows both the correct text-style fraction and the incorrect rendering without \text{...}.

```latex
\documentclass{article}
% Using the geometry package to reduce
% the width of help article graphics
\usepackage[textwidth=8cm]{geometry}
\usepackage{amsmath}% For the \text{...} command
\begin{document}
We use the \texttt{amsmath} package command
\verb|\text{...}| to create text-only fractions
like this:

\[\frac{\text{numerator}}{\text{denominator}}\]

Without the \verb|\text{...}| command the result 
looks like this:

\[\frac{numerator}{denominator}\]
\end{document}
```

--------------------------------

### Basic xcolor Package Usage

Source: https://www.overleaf.com/learn/latex/Using_colours_in_LaTeX

Demonstrates importing the xcolor package and applying a color to an itemize environment and a horizontal rule. The color command is scoped locally using group delimiters.

```latex
\documentclass{article}
\usepackage{xcolor}
\begin{document}
This example shows some instances of using the \texttt{xcolor} package 
to change the color of elements in \LaTeX.

\begin{itemize}
\color{blue}
\item First item
\item Second item
\end{itemize}

\noindent
{\color{red} \rule{\linewidth}{0.5mm}}
\end{document}
```

--------------------------------

### Complete LaTeX Document Example

Source: https://www.overleaf.com/learn/latex/Articles/How_does_LaTeX_typeset_headers_and_footers%3F

A full LaTeX document demonstrating the use of class 10 marks for headers and footers. It includes the definition of \domark and the redefinition of \leftmark and \rightmark, along with sample content across multiple pages to show mark behavior.

```tex
\documentclass{book}
% A short command to use marks with class 10
% and following LaTeX's mark structure {left}{right}
\newcommand{\domark}[2]{\marks 10{{#1}{#2}}}
% Redefine \leftmark and \rightmark to use 
% \botmarks10 and \firstmarks10 respectively
\catcode`@=11
\renewcommand{\leftmark}{\expandafter\@leftmark\botmarks10 \@empty\@empty{} (via \texttt{\string\botmarks10})}
\renewcommand{\rightmark}{\expandafter\@rightmark\firstmarks10 \@empty\@empty{} (via \texttt{\string\firstmarks10})}
\catcode`@=12
\title{Demonstrating \(\varepsilon\)-\TeX’s enhanced marks}
\author{Overleaf}
\date{August 2022}
\begin{document}

Page 1: No marks added so all mark variables remain in their initialized state: empty (NULL).

\newpage
Page 2: $\alpha$-mark added to this page via

\verb|\domark{$\alpha$-left}{$\alpha$-right}|\domark{$\alpha$-left}{$\alpha$-right}

\newpage
Page 3: No new marks added to this page.

\newpage
Page 4: $\beta$-mark followed by $\gamma$-mark added to this page via

\verb|\domark{$\beta$-left}{$\beta$-right}|

\verb|\domark{$\gamma$-left}{$\gamma$-right}|.
\domark{$\beta$-left}{$\beta$-right}
\domark{$\gamma$-left}{$\gamma$-right}

\newpage
Page 5: $\delta$-mark added to this page via

\verb|\domark{$\delta$-left}{$\delta$-right}|
\domark{$\delta$-left}{$\delta$-right}

\newpage
Page 6: No marks added to this page.
\end{document}
```

--------------------------------

### Full Document Example with Custom Page Size

Source: https://www.overleaf.com/learn/latex/Page_size_and_margins

A complete LaTeX document demonstrating the use of the geometry package to set A4 paper size and a custom text area of 6 inches wide and 8 inches high. Includes dummy text for content.

```latex
\documentclass{article}
\usepackage{blindtext}
\usepackage[a4paper, total={6in, 8in}]{geometry}

\begin{document}
\section{Introduction}
This is a test document which uses A4-sized paper and the user-defined text area. 
\subsection{Some dummy text}
\blindtext[8]

\end{document}
```

--------------------------------

### Adjusting Column Separation

Source: https://www.overleaf.com/learn/latex/Multiple_columns

The separation between columns can be set using \setlength{\columnsep}{<length>}. For example, \setlength{\columnsep}{1cm} sets the separation to 1cm.

```latex
\documentclass{article}
\usepackage{blindtext}
\usepackage{multicol}
\setlength{\columnsep}{1cm}
\title{Second multicols Demo}
\author{Overleaf}
\date{April 2021}

\begin{document}
\maketitle

\begin{multicols}{2}
[
\section{First Section}
All human things are subject to decay. And when fate summons, Monarchs must obey.
]
\blindtext\blindtext
\end{multicols}

\end{document}

```

--------------------------------

### LaTeX document with \directlua

Source: https://www.overleaf.com/learn/latex/Articles/An_Introduction_to_LuaTeX_%28Part_2%29%3A_Understanding_%5Cdirectlua

This example shows how to embed \directlua code within a standard LaTeX document structure. It includes the necessary \documentclass and \begin{document}/\end{document} commands.

```tex
\documentclass{article}
\begin{document}
   \directlua{local x=[[\$150 for the "\#1" product---20\%! more than its competitor, Widget \& Co.]] texio.write(x)}
\end{document}
```

--------------------------------

### LuaTeX Usage and Options

Source: https://www.overleaf.com/learn/latex/Articles/The_two_modes_of_TeX_engines%3A_INI_mode_and_production_mode

This is the general usage information for LuaTeX, detailing how to run it with Lua scripts, TeX files, or format files. It outlines various command-line options for controlling its behavior.

```bash
Usage: luatex --lua=FILE [OPTION]... [TEXNAME[.tex]] [COMMANDS]
   or: luatex --lua=FILE [OPTION]... \FIRST-LINE
   or: luatex --lua=FILE [OPTION]... &FMT ARGS
  Run LuaTeX on TEXNAME, usually creating TEXNAME.pdf.
  Any remaining COMMANDS are processed as luatex input, after TEXNAME is read.

  Alternatively, if the first non-option argument begins with a backslash,
  luatex interprets all non-option arguments as an input line.

  Alternatively, if the first non-option argument begins with a &, the
  next word is taken as the FMT to read, overriding all else.  Any
  remaining arguments are processed as above.

  If no arguments or options are specified, prompt for input.

  The following regular options are understood:

   --credits                     display credits and exit
   --debug-format                enable format debugging
   --draftmode                   switch on draft mode (generates no output PDF)
   --[no-]file-line-error        disable/enable file:line:error style messages
   --[no-]file-line-error-style  aliases of --[no-]file-line-error
   --fmt=FORMAT                  load the format file FORMAT
   --halt-on-error               stop processing at the first error
   --help                        display help and exit
   --ini                         be iniluatex, for dumping formats
   --interaction=STRING          set interaction mode (STRING=batchmode/nonstopmode/scrollmode/errorstopmode)
   --jobname=STRING              set the job name to STRING
   --kpathsea-debug=NUMBER       set path searching debugging flags according to the bits of NUMBER
   --lua=FILE                    load and execute a lua initialization script
   --[no-]mktex=FMT              disable/enable mktexFMT generation (FMT=tex/tfm)
   --nosocket                    disable the lua socket library
   --output-comment=STRING       use STRING for DVI file comment instead of date (no effect for PDF)
   --output-directory=DIR        use existing DIR as the directory to write files in
   --output-format=FORMAT        use FORMAT for job output; FORMAT is 'dvi' or 'pdf'
   --progname=STRING             set the program name to STRING
   --recorder                    enable filename recorder
   --safer                       disable easily exploitable lua commands
   --[no-]shell-escape           disable/enable system commands
   --shell-restricted            restrict system commands to a list of commands given in texmf.cnf
   --synctex=NUMBER              enable synctex (see man synctex)
   --utc                         init time to UTC
   --version                     display version and exit

Alternate behaviour models can be obtained by special switches

  --luaonly                      run a lua file, then exit
  --luaconly                     byte-compile a lua file, then exit
  --luahashchars                 the bits used by current Lua interpreter for strings hashing

See the reference manual for more information about the startup process.

Email bug reports to dev-luatex@ntg.nl.
```

--------------------------------

### Document using the jlreq class with LuaLaTeX

Source: https://www.overleaf.com/learn/latex/Japanese%23The_pTeX_engine

Example of a Japanese document using the `jlreq` document class, which requires LuaLaTeX, pLaTeX, or upLaTeX as the compiler.

```latex
\documentclass{jlreq}
\begin{document}
\section{これは最初のセクションである}
日本語で \LaTeX の組版を実証するための導入部分。

フォントはまた、数学的な形態および他の環境で使用することができる
\end{document}
```

--------------------------------

### Basic LaTeX document with colored text using dvisvgm driver

Source: https://www.overleaf.com/learn/latex/Using_colours_in_LaTeX

A complete LaTeX document example demonstrating the use of named colors with the 'dvisvgm' driver. This code will produce an SVG file when compiled.

```latex
\documentclass{article}
\usepackage[dvisvgm, usenames, dvipsnames]{color}
\title{Creating SVG graphics}
\author{Overleaf}
\begin{document}
\maketitle
Hello, {\color{Apricot}in Apricot} and now in {\color{DarkOrchid} DarkOrchid} but perhaps it might look nicer if we use {\color{JungleGreen}JungleGreen}---or may not?
\end{document}
```

--------------------------------

### Book Class: Preliminary and Main Matter Page Numbering

Source: https://www.overleaf.com/learn/latex/Page_numbering

This example uses the 'book' document class to demonstrate traditional page numbering for books, employing Roman numerals for preliminary pages (front matter) and Arabic numerals for the main body pages (main matter). The \emptypage package is included to prevent headers/footers on empty pages.

```latex
\documentclass{book}
% The emptypage package prevents page numbers and
% headings from appearing on empty pages.
\usepackage{emptypage}
\begin{document}
\frontmatter %Use lowercase Roman numerals for page numbers
\chapter*{Foreword}
\addcontentsline{toc}{chapter}{Foreword}
The Foreword is written by someone who is not the book's author.

\chapter*{Preface}
\addcontentsline{toc}{chapter}{Preface}
The Preface is written by the book's author.

\tableofcontents

\mainmatter % Now Use Arabic numerals for page numbers

\chapter{First Chapter}
This will be an empty chapter...
\section{First section}
Some text would be good.
\chapter{The second chapter}
\end{document}
```

--------------------------------

### Accessing TeX Box Width with LuaTeX

Source: https://www.overleaf.com/learn/latex/Articles/Pandora%E2%80%99s_%5Chbox%3A_Using_LuaTeX_to_Lift_the_Lid_of_TeX_Boxes

This example demonstrates how to retrieve the width of a TeX box using LuaTeX's direct access to internal data structures. It compares the result with the traditional TeX method.

```latex
\documentclass{article}
\begin{document}
\setbox0=\hbox{A\hskip 5pt B\hskip 10pt C}
\fontsize{18}{22}\selectfont
\noindent Using \TeX{} code, box 0 has width \number\wd0\relax \space sp\par
\noindent We can also use Lua and call one of Lua\TeX's functions to get the same
information.\vskip10mm
\noindent From Lua code, box 0 has width 
\directlua{
local boxwidth = tex.box[0].width
tex.print(boxwidth.." sp")
} which, of course, is identical to the value obtained from \TeX{} code.
\end{document}

```

--------------------------------

### Set Page Numbering Style

Source: https://www.overleaf.com/learn/latex/Headers_and_footers

Customize the style of page numbers using the \pagenumbering command. For example, use 'roman' for lowercase Roman numerals.

```latex
\pagenumbering{⟨numberstyle⟩}
```

```latex
\pagenumbering{roman}
```

--------------------------------

### LaTeX Document with Custom Paragraph Spacing

Source: https://www.overleaf.com/learn/latex/Articles/How_to_change_paragraph_spacing_in_LaTeX

A complete LaTeX document example that sets custom paragraph spacing parameters and includes a test paragraph. This demonstrates how to apply \baselineskip, \lineskip, and \lineskiplimit within a document structure.

```latex
\documentclass{article}
% Use a conveniently small page size
\usepackage[paperheight=16cm,paperwidth=12cm,textwidth=8cm]{geometry}
% Set some important parameters
\setlength{\baselineskip}{12pt}
\setlength{\lineskip}{3.5pt}
\setlength{\lineskiplimit}{2pt}
\setlength{\parindent}{20pt}
% Input file defining \testpar 
\input testpar.tex
\title{A sample paragraph for lineskip}
\begin{document}
\testpar % A macro created in the Overleaf project
\end{document}

```

--------------------------------

### BibTeX @incollection Entry Example

Source: https://www.overleaf.com/learn/latex/Bibliography_management_with_bibtex

Use @incollection for a contributed chapter in an edited book. Include 'author', 'title', 'booktitle', 'publisher', and optionally 'editor'.

```bibtex
@incollection{Mihalcea:2006,
  author = {Rada Mihalcea},
  title = {Knowledge-Based Methods for {WSD}},
  booktitle = {Word Sense Disambiguation: Algorithms
               and Applications},
  publisher = {Springer},
  year = {2006},
  editor = {Eneko Agirre and Philip Edmonds},
  pages = {107--132},
  address = {Dordrecht, the Netherlands}
}


```

--------------------------------

### BibTeX @inbook Entry Example

Source: https://www.overleaf.com/learn/latex/Bibliography_management_with_bibtex

Use @inbook for a chapter within a book authored by the same person(s). Specify the 'chapter' number along with standard fields.

```bibtex
@inbook{peyret2012:ch7,
  title={Computational Methods for Fluid Flow},
  edition={2},
  author={Peyret, Roger and Taylor, Thomas D},
  year={1983},
  publisher={Springer-Verlag},
  address={New York},
  chapter={7, 14}
}


```

--------------------------------

### Load Base Class and Packages

Source: https://www.overleaf.com/learn/latex/Writing_your_own_class

Extend an existing class like 'article' and include necessary packages using `\LoadClass` and `\RequirePackage`. `\RequirePackage` is recommended over `\usepackage` within class files.

```latex
\NeedsTeXFormat{LaTeX2e}
\ProvidesClass{exampleclass}[2014/08/16 Example LaTeX class]

\newcommand{\headlinecolor}{\normalcolor}
\LoadClass[twocolumn]{article}
\RequirePackage{xcolor}
\definecolor{slcolor}{HTML}{882B21}

```

--------------------------------

### BibTeX @phdthesis Entry Example

Source: https://www.overleaf.com/learn/latex/Bibliography_management_with_bibtex

Use the @phdthesis entry type for doctoral dissertations. Essential fields include 'title', 'school', 'author', and 'year'.

```bibtex
@phdthesis{Alsolami:2012,
    title    = {An examination of keystroke dynamics
                for continuous user authentication},
    school   = {Queensland University of Technology},
    author   = {Eesa Alsolami},
    year     = {2012}
}


```

--------------------------------

### Enable SyncTeX Primitive

Source: https://www.overleaf.com/learn/latex/MLTeX_EncTeX_and_SyncTeX_TeX_extensions

Add this command to your .tex file to enable SyncTeX functionality.

```tex
\synctex=1
```

--------------------------------

### Loading Named Colors with the color Package

Source: https://www.overleaf.com/learn/latex/Using_colours_in_LaTeX

Demonstrates loading named colors using the 'usenames' and 'dvipsnames' options with the standard 'color' package. This is an alternative to using 'xcolor'.

```latex
\usepackage[usenames,dvipsnames]{color}
```

--------------------------------

### Defining a Macro for \uppercase Example

Source: https://www.overleaf.com/learn/latex/Articles/How_does_%5Cexpandafter_work%3A_From_basic_principles_to_exploring_TeX%27s_source_code

Defines a simple macro \foo containing lower-case text. This is used to show that \uppercase does not expand macros by default.

```tex
\def\foo{some lower-case text}

```

--------------------------------

### Defining a Minimal Page Style

Source: https://www.overleaf.com/learn/latex/Articles/How_does_LaTeX_typeset_headers_and_footers%3F

This example defines a custom page style named `demostyle`. It redefines the internal LaTeX commands for headers and footers to use static text and the page number. Note the use of `\catcode` to allow '@' in macro names.

```latex
\documentclass[twoside]{article}
\catcode`@=11
\newcommand{\ps@demostyle}{
\renewcommand\@oddfoot{\hfil The odd-page footer\hfil}
\renewcommand\@evenfoot{\hfil The even-page footer\hfil}
\renewcommand\@evenhead{\thepage\hfil The even-page header}
\renewcommand\@oddhead{The odd-page header\hfil\thepage}}
\catcode`@=12
\title{Demonstrating a page style}
\author{Overleaf}
\date{August 2022}
\begin{document}
\pagestyle{demostyle}
\maketitle
\newpage
\section{Introduction}
\newpage
\section{More material}
\end{document}

```

--------------------------------

### Configure Hindi and Sanskrit with Devanagari Script

Source: https://www.overleaf.com/learn/latex/Multilingual_typesetting_on_Overleaf_using_polyglossia_and_fontspec

Set the default language to English, other languages to Hindi and Sanskrit, and define a font for the Devanagari script. Includes examples of typesetting Hindi and Sanskrit text.

```latex
\setdefaultlanguage{english}
\setotherlanguages{hindi,sanskrit}
\newfontfamily\devanagarifont[Script=Devanagari]{Lohit Devanagari}
... 
Hindi: \texthindi{हिन्दी}
Sanskrit: \textsanskrit{संस्कृतम्}
```

--------------------------------

### Tabular Environment Cell and Row Commands

Source: https://www.overleaf.com/learn/latex/Tables

Shows commands for structuring table content, including cell separation, starting new rows, adding horizontal lines, and creating new lines within cells.

```latex
& \\ \\ [6pt] \hline \newline \cline{i-j}
```

--------------------------------

### Basic 2D and 3D Plot Example with Externalization

Source: https://www.overleaf.com/learn/latex/Pgfplots_package

A basic LaTeX document demonstrating the creation of both 2D and 3D plots using the pgfplots package, with figures externalized for faster compilation. Ensure \\usepackage{pgfplots} and \\pgfplotsset are included.

```latex
\documentclass{article}
\usepackage[margin=0.25in]{geometry}
\usepackage{pgfplots}
\pgfplotsset{width=10cm,compat=1.9}

% We will externalize the figures
\usepgfplotslibrary{external}
\tikzexternalize

\begin{document}

First example is 2D and 3D math expressions plotted side-by-side.

%Here begins the 2D plot
\begin{tikzpicture}
\begin{axis}
\addplot[color=red]{exp(x)};
\end{axis}
\end{tikzpicture}
%Here ends the 2D plot
\hskip 5pt
%Here begins the 3D plot
\begin{tikzpicture}
\begin{axis}
\addplot3[
    surf,
]
{exp(-x^2-y^2)*x};
\end{axis}
\end{tikzpicture}
%Here ends the 3D plot

\end{document}
```

--------------------------------

### Assign Custom Category Codes for Grouping

Source: https://www.overleaf.com/learn/latex/How%20TeX%20macros%20actually%20work%3A%20Part%204

Assigns '(' as the start group character and ')' as the end group character. This allows for custom delimiters in macro definitions.

```tex
\catcode`\(=1
\catcode`\)=2

```

--------------------------------

### Create Questions and Bonus Parts in LaTeX

Source: https://www.overleaf.com/learn/latex/Typesetting_exams_in_LaTeX

Use \question and \part for regular questions and \bonuspart and \bonusquestion for bonus elements. \vspace{\stretch{1}} adds vertical spacing. \droptotalpoints displays the total points for a question.

```latex
\begin{questions}

\question Given the equation \(x^n + y^n = z^n\) for \(x,y,z\) and \(n\) positive
integers. 
\begin{parts}
\part[5] For what values of \(n\) is the statement in the previous question true?
\vspace{\stretch{1}}

\part[2 \half] For \(n=2\) there's a theorem with a special name. What's that name?
\vspace{\stretch{1}}

\bonuspart[2 \half] What famous mathematician had an elegant proof for this theorem but there was
not enough space in the margin to write it down?
\vspace{\stretch{1}}

\end{parts}

\droptotalpoints

\question[20] Compute \[
\int_{0}^{\infty} \frac{\sin(x)}{x}\]

\vspace{\stretch{1}}

\bonusquestion[30] Prove that the real part of all non-trivial zeros of the function 
\(\zeta(z)\) is \(\frac{1}{2}\)
\vspace{\stretch{1}}

\end{questions}
```

--------------------------------

### Handle Row Breaks in tabular Environments

Source: https://www.overleaf.com/learn/how-to/Fixing_and_preventing_compile_timeouts

If tabular rows start with '[...', add \relax after the \\ on the previous row to prevent potential issues.

```latex
\\ \relax
```

--------------------------------

### Grouping Nomenclature Entries with ifthen

Source: https://www.overleaf.com/learn/latex/Nomenclatures

An alternative method for grouping nomenclature entries using the `ifthen` package and its `\ifthenelse` command, achieving the same result as the `etoolbox` example.

```latex
\usepackage{ifthen}
  \renewcommand{\nomgroup}[1]{\item[\bfseries
  \ifthenelse{\equal{#1}{P}}{Physics constants}{
  \ifthenelse{\equal{#1}{O}}{Other symbols}{
  \ifthenelse{\equal{#1}{N}}{Number sets}{}}}
  ]}


```

--------------------------------

### Execute external program and capture output with LuaTeX

Source: https://www.overleaf.com/learn/latex/Articles/Using_LuaTeX_to_run_tools_and_utilities_installed_on_Overleaf%E2%80%99s_servers

This Lua script, embedded within a LaTeX document using `\directlua`, executes a command-line program (`dvisvgm --help` in this example), captures its standard output, and saves it to a file named `command.txt`. The captured text is then included in the LaTeX document using `\verbatiminput`.

```tex
\documentclass{article}
\usepackage{verbatim}
\begin{document}
\directlua{ 
function runcommand(cmd) 
local fout = assert(io.popen(cmd, 'r')) 
local str = assert(fout:read('*a')) 
fout:close()
return str 
end 

local sout=runcommand("dvisvgm --help") 
local marg = assert(io.open("command.txt","w")) 
marg:write(sout)
marg:flush()
marg:close()
} 
\verbatiminput{command.txt} 
\end{document}
```

--------------------------------

### Attempting Uppercase Conversion on Stored Tokens

Source: https://www.overleaf.com/learn/latex/Articles/What_is_a_TeX_token_list

Illustrates an attempt to re-use stored characters (from a token list or macro) with the \uppercase command. This example highlights a common pitfall where direct application does not yield the expected result.

```tex
\uppercase{\the\toks100}

```

```tex
\uppercase{\mychars}

```

--------------------------------

### Typesetting Urdu with Graphite Shaper in LaTeX

Source: https://www.overleaf.com/learn/latex/Articles/An_overview_of_technologies_supporting_the_use_of_colour_emoji_fonts_in_LaTeX

This example demonstrates advanced text shaping for Urdu using the Graphite shaper. It requires the Awami Nastaliq font and the luaotfload package. Ensure the font file is accessible and the shaper is correctly specified.

```latex
\documentclass{article}
\usepackage{luaotfload}
\begin{document}

\font\urdutest={file:AwamiNastaliq-Regular.ttf:mode=harf;shaper=graphite2} at 100bp
% Technology
\pardir TRT\textdir TRT \urdutest ٹیکنالوجی

\vskip 75bp

% Educational
\pardir TRT\textdir TRT \urdutest تعلیمی
\end{document}

```

--------------------------------

### Display XeTeX Command Line Options

Source: https://www.overleaf.com/learn/latex/TeX_engine_command_line_options_for_pdfTeX%2C_XeTeX_and_LuaTeX

Run this command to view a comprehensive list of available command-line options for XeTeX. This helps in understanding and utilizing its various functionalities.

```bash
xetex --help
```

--------------------------------

### Wrap Text Around Figures

Source: https://www.overleaf.com/learn/latex/Inserting_Images%23Positioning

Example demonstrating how to wrap text around figures placed on the right and left sides of the page using the `wrapfigure` environment.

```latex
\begin{wrapfigure}{r}{0.25\textwidth} %this figure will be at the right
    \centering
    \includegraphics[width=0.25\textwidth]{mesh}
\end{wrapfigure}

There are several ways to plot a function of two variables, 
depending on the information you are interested in. For 
instance, if you want to see the mesh of a function so it 
easier to see the derivative you can use a plot like the 
one on the left.


\begin{wrapfigure}{l}{0.25\textwidth}
    \centering
    \includegraphics[width=0.25\textwidth]{contour}
\end{wrapfigure}

On the other side, if you are only interested on
certain values you can use the contour plot, you 
can use the contour plot, you can use the contour 
plot, you can use the contour plot, you can use the 
contour plot, you can use the contour plot, 
like the one on the left.

On the other side, if you are only interested on 
certain values you can use the contour plot, you 
can use the contour plot, you can use the contour 
plot, you can use the contour plot, you can use the 
contour plot, you can use the contour plot, 
you can use the contour plot, 
like the one on the left.

```

--------------------------------

### Initialize Board with FEN Notation

Source: https://www.overleaf.com/learn/latex/Chess_notation

Use the \'setfen\' key within the \chessboard command to initialize the board in any desired position using Forsyth-Edwards Notation (FEN).

```latex
\documentclass{article}
\usepackage{xskak}
\begin{document}
\newchessgame
\chessboard[setfen=r5k1/1b1p1ppp/p7/1p1Q4/2p1r3/PP4Pq/BBP2b1P/R4R1K w - - 0 20]
\end{document}
```

--------------------------------

### Using Google Noto Fonts with XeLaTeX

Source: https://www.overleaf.com/learn/latex/XeLaTeX

This example demonstrates how to use the 'noto' package to typeset a document using Google's Noto Serif, Noto Sans, and Noto Sans Mono font families. It requires the XeLaTeX or LuaLaTeX compiler.

```latex
\documentclass{article}
\usepackage{xcolor}
\usepackage{noto}
\usepackage{hyperref}
\title{Using Google Noto fonts}
\author{Overleaf}
\date{April 2021}

\begin{document}

\maketitle

\section{Introduction}
This example project uses the \href{https://ctan.org/pkg/noto?lang=en}{\color{blue}\texttt{noto}} package to typeset your document using Google's Noto fonts\footnote{\url{https://www.google.com/get/noto/}}:
\begin{itemize}
\item \verb|\textbf{bold}| produces \textbf{bold}
\item \verb|\textit{italic}| produces \textit{italic}
\item \verb|\textbf{\textit{bold italic}}| produces \textbf{\textit{bold italic}}
\item \verb|\emph{emphasis}| produces \emph{emphasis}
\item \verb|\textbf{\emph{bold italic}}| produces \textbf{\emph{bold italic}}
\end{itemize}

\subsection{Monospaced fonts}
You can use Noto's monospaced fonts for \texttt{regular} and \texttt{\textbf{bold}} monospaced text.

\subsection{Sans serif fonts}
Here is some \textsf{text is typeset in a sans serif font} together with \textbf{\textsf{text typeset in bold sans serif}}.

\section{Further reading}
Documentation for the \texttt{noto} package can be found in its \href{http://mirrors.ctan.org/fonts/noto/README}{\color{blue}\texttt{readme} file on CTAN}.

\end{document}

```

--------------------------------

### Typeset Chess Boards with \chessboard

Source: https://www.overleaf.com/learn/latex/Chess_notation

Use the \chessboard command to typeset chess boards with package default values. This is the basic command for displaying a board.

```latex
\documentclass{article}
\usepackage{xskak}
\begin{document}
\newchessgame
\mainline{1.e4 e5 2.Nf3 Nc6 3.Bb5}
\chessboard % instead of \showboard

\newchessgame
\mainline{1.e4 e5 2.Nf3 Nc6 3.Bb5 a6}
\chessboard % instead of \showboard
\end{document}
```

--------------------------------

### Defining External Vertices on the Right

Source: https://www.overleaf.com/learn/latex/Feynman_diagrams

Command to define and position external vertices on the right side of the Feynman diagram. 'o1' and 'o2' are example names for these vertices.

```latex
% Creates two vertices on the right called o1 and o2
\fmfright{o1,o2}
```

--------------------------------

### Typesetting Binomial Coefficients with amsmath

Source: https://www.overleaf.com/learn/latex/Fractions_and_Binomials

Demonstrates the basic typesetting of a binomial coefficient using the \binom command from the amsmath package, which also defines the fraction formula.

```latex
\documentclass{article}
\usepackage{amsmath}
\begin{document}
The binomial coefficient, \(\binom{n}{k}\), is defined by the expression:
\[
    \binom{n}{k} = \frac{n!}{k!(n-k)!}
\]
\end{document}
```

--------------------------------

### Defining External Vertices on the Left

Source: https://www.overleaf.com/learn/latex/Feynman_diagrams

Command to define and position external vertices on the left side of the Feynman diagram. 'i1' and 'i2' are example names for these vertices.

```latex
% Creates two vertices on the left called i1 and i2
\fmfleft{i1,i2}
```

--------------------------------

### Multilingual Document Setup with Babel

Source: https://www.overleaf.com/learn/latex/Multilingual_typesetting_on_Overleaf_using_babel_and_fontspec

Configure a LaTeX document to support multiple languages (French, Russian, English, Thai) using the babel package. Specify fonts for different scripts with \babelfont. Use \foreignlanguage for short snippets and \begin{otherlanguage} for longer sections.

```latex
\documentclass[12pt]{article}
\usepackage{geometry}  % to use a small page size
\geometry{margin=4cm,b5paper}
\usepackage[english,russian,french]{babel}
\babelprovide[import]{thai}
\babelfont{rm}{FreeSerif}
\babelfont{sf}{FreeSans}
\babelfont{tt}{FreeMono}
\begin{document}
\begin{abstract}
Le Lorem Ipsum est simplement du faux texte employé dans la composition et la mise en page avant impression.
\end{abstract}
 
Merci. \foreignlanguage{english}{Thank you.} \foreignlanguage{thai}{ขอบคุณ} \foreignlanguage{russian}{Спасибо.} Et plus de
texte en français!
 
Le Lorem Ipsum est le faux texte standard de l'imprimerie depuis les années 1500, quand un imprimeur anonyme assembla ensemble des morceaux de texte pour réaliser un livre spécimen de polices de texte.

\begin{otherlanguage}{english}
Lorem Ipsum is simply dummy text of the printing and typesetting industry. Lorem Ipsum has been the industry’s standard dummy text ever since the 1500s, when an unknown printer took a galley of type and scrambled it to make a type specimen book.

It has survived not only five centuries, but also the leap into electronic typesetting, remaining essentially unchanged. \end{otherlanguage}
 
\begin{otherlanguage}{russian}
Lorem Ipsum - это текст-`\textsf{рыба}', часто используемый в \texttt{печати} и вэб-дизайне. Lorem Ipsum является стандартной ``рыбой'' для текстов на латинице с начала XVI века. В то время некий безымянный печатник создал большую коллекцию размеров и форм шрифтов, используя Lorem Ipsum для распечатки образцов. Lorem Ipsum не только успешно пережил без заметных изменений пять веков, но и перешагнул в электронный дизайн. \end{otherlanguage}
 
\begin{otherlanguage}{thai}
\foreignlanguage{english}{Lorem Ipsum} คือ เนื้อหาจำลองแบบเรียบๆ ที่ใช้กันในธุรกิจงานพิมพ์หรืองานเรียงพิมพ์
\end{otherlanguage}
\end{document}
```

--------------------------------

### Basic Document Structure with \import

Source: https://www.overleaf.com/learn/latex/Management_in_a_large_project%23Importing_files

This snippet demonstrates a typical book structure using the \import command to include various parts of the document like title, chapters, sections, and bibliography from separate files.

```latex
\documentclass[a4paper,11pt]{book}
\usepackage{import}
\usepackage{example}

\usepackage{makeidx}
\makeindex

\begin{document}

\frontmatter
\import{./}{title.tex}

\clearpage
\thispagestyle{empty}

\tableofcontents

\mainmatter
\chapter{First chapter}
\import{sections/}{section1-1.tex}
\import{sections/}{section1-2.tex}

\chapter{Additional chapter}
\import{sections/}{section2-1.tex}

\chapter{Last chapter}
\import{sections/}{section3-1.tex}

\backmatter

\import{./}{bibliography.tex}

\end{document}
```

--------------------------------

### Demonstrate OT1 vs T1 Font Encoding in LaTeX

Source: https://www.overleaf.com/learn/latex/French

This example illustrates the difference between LaTeX's default OT1 encoding (which fakes accented characters) and the T1 encoding (which uses genuine glyphs). Observe the copy-paste behavior from the generated PDF.

```latex
\documentclass{article}
\begin{document}
Section théorèmes (OT1 encoding)

{
\fontencoding{T1}\selectfont Section théorèmes (T1 encoding)
}
\end{document}
```

--------------------------------

### Use Variations and \variation Command

Source: https://www.overleaf.com/learn/latex/Chess_notation

Demonstrate chess variations using the \variation command and the three-dot (...) syntax for black moves. This allows for showing alternative lines of play.

```latex
\documentclass{article}
\usepackage{xskak}
\begin{document}
\newchessgame
\mainline{1.e4 e5 2.Nf3 Nc6 3.Bb5}

\chessboard

\mainline{3...a6}

A variant \variation{3...Nf6} is used here to show a \texttt{\string\variation} command.

\mainline{4.Ba4}

\chessboard
\end{document}
```

--------------------------------

### Define Token Register

Source: https://www.overleaf.com/learn/latex/Articles/What_is_a_%22TeX_token%22%3F

Example of defining a token register in TeX and assigning a string value to it. TeX internally converts this string into a list of character tokens.

```tex
\toks100={Hello}
```

--------------------------------

### Create a Basic LaTeX Table

Source: https://www.overleaf.com/learn/latex/Creating_a_document_in_LaTeX

Use the `tabular` environment to create tables. Specify column alignment (e.g., `c` for center) in the environment's argument. Use `&` to separate cells and `\\` to end rows. Wrap the table in a `center` environment to center it on the page.

```latex
\begin{center}
\begin{tabular}{c c c}
 cell1 & cell2 & cell3 \\ 
 cell4 & cell5 & cell6 \\  
 cell7 & cell8 & cell9    
\end{tabular}
\end{center}
```

--------------------------------

### LaTeX Table with Custom Borders

Source: https://www.overleaf.com/learn/how-to/How_to_insert_tables_in_Overleaf%23editable

This example shows how to create tables with custom borders using the '|' character in the tabular environment's column specification and the '\hline' command within the table body. These borders are previewed in Overleaf's Visual Editor.

```latex
\documentclass{article}
\begin{document}
\begin{table}
\centering
\begin{tabular}{c|c|c}
         R1C1&R1C2& \\
         \hline
         R2C1&  & \\
         \hline
         R3C1 &R3C2 &R3C3\\
\end{tabular}
    \caption{This is my table caption.}
    \label{tab:mynewtable}
\end{table}
\end{document}
```

--------------------------------

### Define a basic theorem environment in LaTeX

Source: https://www.overleaf.com/learn/latex/Theorems_and_proofs

Use \newtheorem to define a new theorem environment. The first argument is the environment name, and the second is the displayed name.

```latex
\newtheorem{theorem}{Theorem}
```

--------------------------------

### LaTeX error for missing graphic file

Source: https://www.overleaf.com/learn/latex/Articles/An_introduction_to_Kpathsea_and_how_TeX_engines_search_for_files

Example of a LaTeX error message indicating that a graphic file could not be found. It lists the extensions TeX searches for.

```text
! LaTeX Error: File `endlinechar' not found.
l.4 \includegraphics{endlinechar}
I could not locate the file with any of these extensions:
.pdf,.PDF,.ai,.AI,.png,.PNG,.jpg,.JPG,.jpeg,.JPEG,.jp2,.JP2,.jpf,.JPF,.bmp,.BMP,
,.ps,.PS,.eps,.EPS,.mps,.MPS,.pz,.eps.Z,.ps.Z,.ps.gz,.eps.gz


```

--------------------------------

### Directly use TeX command to change category code

Source: https://www.overleaf.com/learn/latex/Understanding_TeX_macros%3A_Part_6?preview=true

Shows the correct way to change the category code of a '$' to 11 and then typeset '$90' as regular text. This serves as a comparison to the macro example, highlighting the difference in TeX's processing.

```tex
\begin{document}
I paid \catcode`\$=11 $90 for that book.
\end{document}
```

--------------------------------

### Character Token Calculation Example

Source: https://www.overleaf.com/learn/latex/Articles/How_does_%5Cexpandafter_work%3A_An_introduction_to_TeX_tokens

Demonstrates the calculation of a character token for the letter 'A' with a category code of 11. This formula is fundamental to how TeX represents characters internally.

```tex
character token = 256 * (category code) + character (ASCII) code
Example: The letter A with category code 11, character code 65 is represented by TeX as the character token value 256 * 11 + 65 = 2881.
```

--------------------------------

### Full Multilingual Document Setup with babel and fontspec

Source: https://www.overleaf.com/learn/latex/Multilingual_typesetting_on_Overleaf_using_babel_and_fontspec

A complete LaTeX document demonstrating multilingual typesetting. It sets up multiple languages (French, English, Russian, Thai) and specifies custom fonts for English, Cyrillic, and Thai scripts using \babelfont.

```latex
\documentclass[12pt]{article}
\usepackage{geometry} % to use a small page size
\geometry{margin=4cm,b5paper}
\usepackage[english,russian,main=french]{babel}
\babelprovide[import]{thai}

%% When all the \babelfont lines are uncommented, we can show that e.g. \babelfont[english], [*cyrillic] etc really do override the default \babelfont{rm} for English, Cyrillic.
\babelfont{rm}[Language=Default]{FreeSerif}
\babelfont{sf}[Language=Default]{FreeSans}
\babelfont{tt}[Language=Default]{FreeMono}

\babelfont[english]{rm}{Chancery Uralic}
\babelfont[*cyrillic]{rm}{Charis SIL}
\babelfont[thai]{rm}{Garuda}
\begin{document}
\begin{abstract}
Le Lorem Ipsum est simplement du faux texte employé dans la composition et la mise en page avant impression.
\end{abstract}
 
Merci. \foreignlanguage{english}{Thank you.} \foreignlanguage{thai}{ขอบคุณ} \foreignlanguage{russian}{Спасибо.} Et plus de
texte en français!
 
Le Lorem Ipsum est le faux texte standard de l\'imprimerie depuis les années 1500, quand un imprimeur anonyme assembla ensemble des morceaux de texte pour réaliser un livre spécimen de polices de texte.

\begin{otherlanguage}{english}
Lorem Ipsum is simply dummy text of the printing and typesetting industry. Lorem Ipsum has been the industry\'s standard dummy text ever since the 1500s, when an unknown printer took a galley of type and scrambled it to make a type specimen book. 

It has survived not only five centuries, but also the leap into electronic typesetting, remaining essentially unchanged. \end{otherlanguage}
 
\begin{otherlanguage}{russian}
Lorem Ipsum - это текст-`\textsf{рыба}', часто используемый в \texttt{печати} и вэб-дизайне. Lorem Ipsum является стандартной ``рыбой'' для текстов на латинице с начала XVI века. В то время некий безымянный печатник создал большую коллекцию размеров и форм шрифтов, используя Lorem Ipsum для распечатки образцов. Lorem Ipsum не только успешно пережил без заметных изменений пять веков, но и перешагнул в электронный дизайн. \end{otherlanguage}
 
\begin{otherlanguage}{thai}
\foreignlanguage{english}{Lorem Ipsum} คือ เนื้อหาจำลองแบบเรียบๆ ที่ใช้กันในธุรกิจงานพิมพ์หรืองานเรียงพิมพ์
\end{otherlanguage}
\end{document}
```

--------------------------------

### User-Defined Binary and Relational Operator Spacing

Source: https://www.overleaf.com/learn/latex/Spacing_in_math_mode

Demonstrates how to explicitly define spacing for custom operators using \mathbin for binary operators and \mathrel for relational operators.

```latex
\begin{align*}
34x^2a \mathbin{\#} 13bc \\
34x^2a \mathrel{\#} 13bc
\end{align*}
```

--------------------------------

### Setting Zero Offsets

Source: https://www.overleaf.com/learn/latex/Articles/A_visual_guide_to_LaTeX%E2%80%99s_page_layout_parameters

Initialize \hoffset and \voffset to 0mm for predictable page layout calculations. \pdfhorigin and \pdfvorigin can be set to define the origin if needed.

```latex
\hoffset=0mm
\voffset=0mm
\pdfhorigin=1in
\pdfvorigin=1in
```

--------------------------------

### LaTeX Abstract Environment

Source: https://www.overleaf.com/learn/latex/Learn_LaTeX_in_30_minutes

Demonstrates how to typeset an abstract for a scientific article using LaTeX's `abstract` environment.

```latex
\documentclass{article}
\begin{document}
\begin{abstract}
This is a simple paragraph at the beginning of the 
document. A brief introduction about the main subject.
\end{abstract}
\end{document}
```

--------------------------------

### Highlight Squares and Fields

Source: https://www.overleaf.com/learn/latex/Chess_notation

Use `markfields`, `color`, `colorbackfield`, and `markfield` to highlight specific squares and regions on the chessboard. Supports FEN notation for board setup.

```latex
\documentclass{article}
\usepackage{xskak}
\begin{document}
\newgame
\chessboard[setfen=8/8/8/3Q4/8/8/8/8 w - - 0 0,
            pgfstyle=border,markfields={d4,d6},
            color=blue!50,
            colorbackfield=c5,
            pgfstyle=color,
            opacity=0.5,
            color=red,
            markfield={d5}]
\end{document}
```

--------------------------------

### Compile to DVI Format

Source: https://www.overleaf.com/learn/latex/Choosing_a_LaTeX_Compiler%23Other_compilers

Use this command to compile a LaTeX document into a DVI file. This format is device-independent.

```bash
latex mydocument.tex
```

--------------------------------

### Select Plain TeX Engine in latexmkrc

Source: https://www.overleaf.com/learn/latex/Questions/Can_I_run_plain_TeX_on_Overleaf%3F

Uncomment the desired line to select a specific TeX engine for compilation. For example, uncommenting the last line enables XeTeX.

```latexmkrc
# $latex = 'tex %O %S'; # to use Knuth's original TeX engine
# $latex = 'pdftex %O %S'; # to use the pdfTeX engine
# $latex = 'luatex %O %S'; # to use the LuaTeX engine
# $latex = 'xetex %O %S';  # to use the XeTeX engine

```

```latexmkrc
# $latex = 'tex %O %S'; # to use Knuth's original TeX engine
# $latex = 'pdftex %O %S'; # to use the pdfTeX engine
# $latex = 'luatex %O %S'; # to use the LuaTeX engine
$latex = 'xetex %O %S';  # to use the XeTeX engine

```

--------------------------------

### Basic LaTeX Document Structure and Math

Source: https://www.overleaf.com/learn/latex/Learn_LaTeX_in_30_minutes

Demonstrates basic LaTeX document structure with article class, including math mode for subscripts, superscripts, integrals, fractions, and Greek letters. Shows how to use display math with \[...\] and inline math with $...$.

```latex
\documentclass{article}
\begin{document}
Subscripts in math mode are written as $a_b$ and superscripts are written as $a^b$. These can be combined and nested to write expressions such as

\[ T^{i_1 i_2 \dots i_p}_{j_1 j_2 \dots j_q} = T(x^{i_1},\dots,x^{i_p},e_{j_1},\dots,e_{j_q}) \]
 
We write integrals using $\int$ and fractions using $\frac{a}{b}$. Limits are placed on integrals using superscripts and subscripts:

\[ \int_0^1 \frac{dx}{e^x} =  \frac{e-1}{e} \]

Lower case Greek letters are written as $\omega$ $\delta$ etc. while upper case Greek letters are written as $\Omega$ $\Delta$.

Mathematical operators are prefixed with a backslash as $\sin(\beta)$, $\cos(\alpha)$, $\log(x)$ etc.
\end{document}
```

--------------------------------

### Specify Fonts for Specific Languages/Scripts

Source: https://www.overleaf.com/learn/latex/Multilingual_typesetting_on_Overleaf_using_babel_and_fontspec

Use \babelfont with language or script names (preceded by *) to assign specific fonts. This example sets fonts for English, Cyrillic, and Thai.

```latex
\babelfont[english]{rm}{Chancery Uralic}
\babelfont[*cyrillic]{rm}{Charis SIL}
\babelfont[thai]{rm}{Garuda}
```

--------------------------------

### Article Class: Custom Page Numbering and TOC

Source: https://www.overleaf.com/learn/latex/Page_numbering

This example demonstrates setting page numbering to roman, manually adjusting page counters, and adding entries to the table of contents with estimated page counts. It concludes by switching to arabic page numbering.

```latex
\documentclass{article}
\pagenumbering{roman}
\begin{document}

\section*{Guest Foreword (TBD)}
\addcontentsline{toc}{section}{Foreword: 4 pages (TBD)}
To be written: Allowing 4 pages.
\newpage 
\setcounter{page}{5}

\section*{Introduction (2 pages)}
\addcontentsline{toc}{section}{Introduction: 2 pages (TBD)}
To be written: 2 pages allowed.
\newpage
\addtocounter{page}{1}

\section*{Strategy summary (2 pages)}
\addcontentsline{toc}{section}{Strategy summary: 2 pages (TBD)}
To be written: 2 pages.
\newpage
\stepcounter{page}

\tableofcontents

\newpage
\pagenumbering{arabic}

\section{Level 1 heading}
Some text on this page.
\newpage
\section{Another Level 1 heading}
\end{document}
```

--------------------------------

### pdfTeX Usage and Options

Source: https://www.overleaf.com/learn/latex/TeX_engine_command_line_options_for_pdfTeX%2C_XeTeX_and_LuaTeX

This block outlines the general usage of pdfTeX and lists its command line options. Options control behavior such as draft mode, interaction levels, and output formats.

```text
Usage: pdftex [OPTION]... [TEXNAME[.tex]] [COMMANDS]
   or: pdftex [OPTION]... \FIRST-LINE
   or: pdftex [OPTION]... &FMT ARGS
  Run pdfTeX on TEXNAME, usually creating TEXNAME.pdf.
  Any remaining COMMANDS are processed as pdfTeX input, after TEXNAME is read.
  If the first line of TEXNAME is %&FMT, and FMT is an existing .fmt file,
  use it.  Else use `NAME.fmt', where NAME is the program invocation name,
  most commonly `pdftex'.

  Alternatively, if the first non-option argument begins with a backslash,
  interpret all non-option arguments as a line of pdfTeX input.

  Alternatively, if the first non-option argument begins with a &, the
  next word is taken as the FMT to read, overriding all else.  Any
  remaining arguments are processed as above.

  If no arguments or options are specified, prompt for input.

-draftmode              switch on draft mode (generates no output PDF)
-enc                    enable encTeX extensions such as \mubyte
-etex                   enable e-TeX extensions
[-no]-file-line-error   disable/enable file:line:error style messages
-fmt=FMTNAME            use FMTNAME instead of program name or a %& line
-halt-on-error          stop processing at the first error
-ini                    be pdfinitex, for dumping formats; this is implicitly
                          true if the program name is `pdfinitex'
-interaction=STRING     set interaction mode (STRING=batchmode/nonstopmode/
                          scrollmode/errorstopmode)
-ipc                    send DVI output to a socket as well as the usual
                          output file
-ipc-start              as -ipc, and also start the server at the other end
-jobname=STRING         set the job name to STRING
-kpathsea-debug=NUMBER  set path searching debugging flags according to
                          the bits of NUMBER
[-no]-mktex=FMT         disable/enable mktexFMT generation (FMT=tex/tfm/pk)
-mltex                  enable MLTeX extensions such as \charsubdef
-output-comment=STRING  use STRING for DVI file comment instead of date
                          (no effect for PDF)
-output-directory=DIR   use existing DIR as the directory to write files in
-output-format=FORMAT   use FORMAT for job output; FORMAT is `dvi' or `pdf'
[-no]-parse-first-line  disable/enable parsing of first line of input file
-progname=STRING        set program (and fmt) name to STRING
-recorder               enable filename recorder
[-no]-shell-escape      disable/enable \write18{SHELL COMMAND}
-shell-restricted       enable restricted \write18
-src-specials           insert source specials into the DVI file
-src-specials=WHERE     insert source specials in certain places of
                          the DVI file. WHERE is a comma-separated value
                          list: cr display hbox math par parend vbox
-synctex=NUMBER         generate SyncTeX data for previewers according to
                          bits of NUMBER (`man synctex' for details)
-translate-file=TCXNAME use the TCX file TCXNAME
-8bit                   make all characters printable by default
-help                   display this help and exit
-version                output version information and exit

pdfTeX home page: <http://pdftex.org>

Email bug reports to pdftex@tug.org.
```

--------------------------------

### Triggering Warnings in Single-sided Documents

Source: https://www.overleaf.com/learn/latex/Headers_and_footers

This example demonstrates how using 'even' page coordinates (E) in single-sided documents triggers warnings from the fancyhdr package, as these coordinates are not applicable.

```latex
\documentclass{article}
\usepackage[paperheight=16cm, paperwidth=12cm,% Set the height and width of the paper
includehead,
nomarginpar,% We don't want any margin paragraphs
textwidth=10cm,% Set \textwidth to 10cm
headheight=10mm,% Set \headheight to 10mm
]{geometry}
\usepackage{fancyhdr}
\begin{document}
% Set the page style to "fancy"...
\pagestyle{fancy}
\title{Warning without twoside}
\author{Overleaf}
\date{July 2022}
\fancyhead[E]{Hello}% This triggers a warning!
\fancyfoot[E]{\thepage}% This triggers a warning!
\maketitle
\section{Introduction}
Some content.
\newpage
\section{Continued...}
\end{document}
```

--------------------------------

### Basic Python Code Highlighting

Source: https://www.overleaf.com/learn/latex/Code_Highlighting_with_minted

Import the minted package and use the minted environment to highlight Python code. The language is specified as a parameter to the environment.

```latex
\documentclass{article}
\usepackage{minted}
\begin{document}
\begin{minted}{python}
import numpy as np
    
def incmatrix(genl1,genl2):
    m = len(genl1)
    n = len(genl2)
    M = None #to become the incidence matrix
    VT = np.zeros((n*m,1), int)  #dummy variable
    
    #compute the bitwise xor matrix
    M1 = bitxormatrix(genl1)
    M2 = np.triu(bitxormatrix(genl2),1) 

    for i in range(m-1):
        for j in range(i+1, m):
            [r,c] = np.where(M2 == M1[i,j])
            for k in range(len(r)):
                VT[(i)*n + r[k]] = 1;
                VT[(i)*n + c[k]] = 1;
                VT[(j)*n + r[k]] = 1;
                VT[(j)*n + c[k]] = 1;
                
                if M is None:
                    M = np.copy(VT)
                else:
                    M = np.concatenate((M, VT), 1)
                
                VT = np.zeros((n*m,1), int)
    
    return M
\end{minted}
\end{document}
```

--------------------------------

### Define a custom LaTeX package

Source: https://www.overleaf.com/learn/latex/Management_in_a_large_project

Use this to organize your preamble with custom commands, styles, and package inclusions. Ensure the \ProvidesPackage command matches the filename for correct import.

```latex
\ProvidesPackage{example}

\usepackage{amsmath}
\usepackage{amsfonts}
\usepackage{amssymb}
\usepackage[latin1]{inputenc}
\usepackage[spanish, english]{babel}
\usepackage{graphicx}
\usepackage{blindtext}
\usepackage{textcomp}
\usepackage{pgfplots}

\pgfplotsset{width=10cm,compat=1.9}

%Header styles
\usepackage{fancyhdr}
\setlength{\headheight}{15pt}
\pagestyle{fancy}
\renewcommand{\chaptermark}[1]{\markboth{#1}{}}
\renewcommand{\sectionmark}[1]{\markright{#1}{}}
\fancyhf{}
\fancyhead[LE,RO]{\thepage}
\fancyhead[RE]{\textbf{\textit{\nouppercase{\leftmark}}}}}
\fancyhead[LO]{\textbf{\textit{\nouppercase{\rightmark}}}}
\fancypagestyle{plain}{ % 
\fancyhf{} % remove everything
\renewcommand{\headrulewidth}{0pt} % remove lines as well
\renewcommand{\footrulewidth}{0pt}}

%makes available the commands \proof, \qedsymbol and \theoremstyle
\usepackage{amsthm}

%Ruler
\newcommand{\HRule}{\rule{\linewidth}{0.5mm}}

%Lemma definition and lemma counter
\newtheorem{lemma}{Lemma}[section]

%Definition counter
\theoremstyle{definition}
\newtheorem{definition}{Definition}[section]

%Corolary counter
\newtheorem{corolary}{Corolary}[section]

%Commands for naturals, integers, topology, hull, Ball, Disc, Dimension, boundary and a few more
\newcommand{\E}{{\mathcal{E}}}
\newcommand{\F}{{\mathcal{F}}}
...

%Example environment
\theoremstyle{remark}
\newtheorem{examle}{Example}

%Example counter
\newcommand{\reiniciar}{\setcounter{example}{0}}

```

--------------------------------

### Integral with Limits

Source: https://www.overleaf.com/learn/latex/Subscripts_and_superscripts

Use `rac{` and `
ight)` for fractions. The `rac` command requires two arguments: the numerator and the denominator. The `
ight)` command ensures the closing parenthesis scales correctly with the content inside.

```latex
\[ 
  \int\limits_0^1 x^2 + y^2 \ dx 
\]
```

--------------------------------

### Setting Unit Length for Diagram Size

Source: https://www.overleaf.com/learn/latex/Feynman_diagrams

Demonstrates how to change the base unit for diagram dimensions using `\setlength{\unitlength}`. This example sets the unit length to 1cm.

```latex
\documentclass{article}
\usepackage{feynmp-auto}
\begin{document}
\setlength{\unitlength}{1cm}
\begin{fmffile}{first-diagram}
 \begin{fmfgraph}(8,5)% units are now in cm
   \fmfleft{i1,i2}
   \fmfright{o1,o2}
   \fmf{fermion}{i1,v1,o1}
   \fmf{fermion}{i2,v2,o2}
   \fmf{photon}{v1,v2}
 \end{fmfgraph}
\end{fmffile}
\end{document}
```

--------------------------------

### Picture Environment with Shifted Origin

Source: https://www.overleaf.com/learn/latex/Picture_environment

Illustrates how to shift the origin of the \begin{picture} environment using the optional argument (x,y). The bounding box remains unaffected by the origin shift.

```latex
\documentclass{article}
\usepackage[pdftex]{pict2e}
\usepackage[dvipsnames]{xcolor}
\begin{document}
\setlength{\unitlength}{1cm}
\setlength{\fboxsep}{0pt}

This is my picture\fbox{
\begin{picture}(3,3)(1,1)
\put(0,0){{\color{blue}\circle*{0.25}}\hbox{\kern3pt\texttt{(0,0)}}}
\put(1,1){{\color{orange}\circle*{0.25}}\hbox{\kern3pt\texttt{(1,1)}}}
\put(3,3){{\color{red}\circle*{0.25}}\hbox{\kern3pt\texttt{(3,3)}}}
\put(4,4){{\color{black}\circle*{0.25}}\hbox{\kern3pt\texttt{(4,4)}}}
\end{picture}}
\end{document}
```

--------------------------------

### Store Tokens in a Token Register

Source: https://www.overleaf.com/learn/latex/Articles/An_Introduction_to_LuaTeX_%28Part_2%29%3A_Understanding_%5Cdirectlua

Example of using the TeX primitive \toks to save a list of tokens in a specified token register. Tokens are stored without expansion.

```tex
\toks100={Hi, \TeX! \hskip 5bp}
```

--------------------------------

### Complete LaTeX Document with Greek and Latin Text

Source: https://www.overleaf.com/learn/latex/Greek

A full LaTeX document example demonstrating the use of LGR and T1 font encodings, the `babel` package for Greek, and the `alphabeta` package for direct Greek character input in math mode. Includes abstract, sections, and Latin text.

```latex
\documentclass{article}

% Set the font (output) encodings
\usepackage[LGR, T1]{fontenc}

% \usepackage[utf8]{inputenc} is no longer required (since 2018)

% Greek-specific commands
\usepackage[greek]{babel}

% Use Greek characters directly in mathematical mode 
% instead of using the commands \alpha etc
\usepackage{alphabeta}

\begin{document}
\tableofcontents

\begin{abstract}
Αυτή είναι μια σύντομη περιγραφή του θέματος 
sαφέστερα εξηγείται στο παρόν έγγραφο
\end{abstract}

\section{εισαγωγή}
Αυτό είναι το πρώτο τμήμα του εγγράφου. Είναι 
μια εισαγωγική παράγραφος.

\section{δεύτερο τμήμα}
Το δεύτερο τμήμα του εγγράφου. Αυτή η ενότητα 
μπορεί να περιέχει μαθηματική σημειογραφία.

\[x^2 + y^2 - \alpha = 4τ + 5α \]

\textlatin{Latin text can also be added to 
the document.}
\end{document}

```

--------------------------------

### LaTeX Document with Various Math Font Styles

Source: https://www.overleaf.com/learn/latex/Mathematical_fonts

Shows how to use calligraphic, fraktur, and blackboard bold typefaces for capital letters. Ensure `\usepackage{amssymb}` is included in your document preamble.

```latex
\documentclass{article}
\usepackage{amsmath}
\usepackage{amssymb}
\begin{document}
\begin{align*}
RQSZ \\
\mathcal{RQSZ} \\
\mathfrak{RQSZ} \\
\mathbb{RQSZ}
\end{align*}
\end{document}
```

--------------------------------

### LuaHBTeX Successfully Loads Color Emoji Font

Source: https://www.overleaf.com/learn/latex/Articles/An_overview_of_technologies_supporting_the_use_of_colour_emoji_fonts_in_LaTeX

This Plain TeX example, compiled with LuaHBTeX, successfully loads and uses the NotoColorEmoji.ttf font by specifying the HarfBuzz renderer.

```tex
\input luaotfload.sty
\font\emojifont=NotoColorEmoji.ttf:mode=harf at 12pt
\emojifont \Uchar"1F600
\bye

```

--------------------------------

### Load LaTeX Package Without Options

Source: https://www.overleaf.com/learn/latex/Creating_a_document_in_LaTeX

If a package has no options or you want to use default settings, load it directly with \usepackage.

```latex
\usepackage{somepackage}
```

--------------------------------

### Undefined Control Sequence Error in TeX

Source: https://www.overleaf.com/learn/latex/Articles/What_is_a_%22TeX_token%22%3F

Shows an example where a macro defined within a group is used outside its scope, resulting in an 'Undefined control sequence' error.

```tex
{\def\foo{Hello}}% \foo defined within a group (note: no use of \global) 
\foo %<--- no longer defined, now undefined
```

--------------------------------

### Load LaTeX Package with Options

Source: https://www.overleaf.com/learn/latex/Creating_a_document_in_LaTeX

Load a LaTeX package with specific options by enclosing them in square brackets after \usepackage. Options are typically comma-separated.

```latex
\usepackage[options]{somepackage}
```

--------------------------------

### Set Biblatex Citation Style to Alphabetic

Source: https://www.overleaf.com/learn/latex/Biblatex_citation_styles

Use the `style` option within the `biblatex` package to specify the desired citation style. This example sets the style to `alphabetic`.

```latex
\documentclass{article}
\usepackage[
backend=biber,
style=alphabetic,
]{biblatex}
\title{A bibLaTeX example}
\addbibresource{sample.bib} %Imports bibliography file

\begin{document}
\section{First section}

Items that are cited: \textit{The \LaTeX\ Companion} book \cite{latexcompanion} together with Einstein's journal paper \cite{einstein} and Dirac's book \cite{dirac}---which are physics-related items. Next, citing two of Knuth's books: \textit{Fundamental Algorithms} \cite{knuth-fa} and \textit{The Art of Computer Programming} \cite{knuth-acp}.

\medskip

\printbibliography
\end{document}


```

--------------------------------

### Define Simple Custom LaTeX Command

Source: https://www.overleaf.com/learn/latex/Commands%23Defining_a_new_command

Shows how to define a new LaTeX command \R that represents the set of real numbers using blackboard boldface, requiring the amssymb package.

```latex
\documentclass{article}
\usepackage{amssymb}
\begin{document}
\newcommand{\R}{\mathbb{R}}
The set of real numbers are usually represented 
by a blackboard bold capital R: \( \R \).
\end{document}


```

--------------------------------

### Adjust acro package option key for version 2

Source: https://www.overleaf.com/learn/how-to/Why_do_I_keep_getting_the_compile_timeout_error_message%3F

This example demonstrates changing the `include` option key to `include-classes` when using the `acro` package version 2.

```latex
include-classes=...
```

--------------------------------

### Comparing \expandafter Syntax

Source: https://www.overleaf.com/learn/latex/Articles/How_does_%5Cexpandafter_work%3A_From_basic_principles_to_exploring_TeX%27s_source_code

Illustrates the structure of \expandafter commands by comparing a general form (T1T2) with a specific usage involving \jobname.

```tex
\expandafter T1T2
```

```tex
\expandafter{\jobname}
```

--------------------------------

### Redefine LaTeX Class Commands

Source: https://www.overleaf.com/learn/latex/Writing_your_own_class

Customizes the appearance of document elements like the title and sections. This example redefines \maketitle and \section to use a specific headline color and font.

```tex
\renewcommand{\maketitle}{
    \twocolumn[
        \fontsize{50}{60}\fontfamily{phv}\fontseries{b}%
        \fontshape{sl}\selectfont\headlinecolor
        \@title
        \medskip
        ]
}

\renewcommand{\section}{
    \@startsection
    {section}{1}{0pt}{-1.5ex plus -1ex minus -.2ex}%
    {1ex plus .2ex}{\large\sffamily\slshape\headlinecolor}%
}

\renewcommand{\normalsize}{\fontsize{9}{10}\selectfont}
\setlength{\textwidth}{17.5cm}
\setlength{\textheight}{22cm}
\setcounter{secnumdepth}{0}

```

--------------------------------

### Configure Natbib Bibliography Style and Import

Source: https://www.overleaf.com/learn/latex/Natbib_bibliography_styles

Use these commands in the LaTeX preamble to load the natbib package and set the bibliography style. The \bibliography command specifies the .bib file to be used.

```latex
%in the preamble
%--------------------------------
  \usepackage{natbib}
  \bibliographystyle{stylename}
%--------------------------------

%Where the bibliography will be printed
  \bibliography{bibfile}
```

--------------------------------

### Compiling with latexmk for Cross-references

Source: https://www.overleaf.com/learn/latex/Cross_referencing_sections%2C_equations_and_floats

Use the command 'latexmk -pdf main.tex' to compile your LaTeX document and ensure all cross-references are correctly generated. Use '-dvi' or '-ps' to change the output format.

```bash
latexmk -pdf main.tex
```

--------------------------------

### Nomenclature with Table of Contents and Language Options

Source: https://www.overleaf.com/learn/latex/Nomenclatures

This example shows how to integrate the nomenclature list into the table of contents and set a specific language (Spanish) for the nomenclature title and related text.

```latex
\documentclass{article}
\usepackage[spanish]{babel}
\usepackage[intoc, spanish]{nomencl}
\makenomenclature

\begin{document}
\tableofcontents

\section{Primera Sección}

Lorem ipsum dolor sit amet, consectetuer adipiscing elit. Etiam lobortisfacilisis sem. Nullam nec mi et neque pharetra sollicitudin. Praesent imperdie...

\nomenclature{\(c\)}{Speed of light in a vacuum}
\nomenclature{\(h\)}{Planck constant}

\printnomenclature
\end{document}
```

--------------------------------

### TikZposter Body with Blocks and Columns

Source: https://www.overleaf.com/learn/latex/Posters

Demonstrates creating a tikzposter with text blocks, multi-column layouts, and notes. Includes adding figures and using blindtext for placeholder content.

```latex
\documentclass[25pt, a0paper, portrait]{tikzposter}

\title{Tikz Poster Example}
\author{Overleaf Team}
\date{\today}
\institute{Overleaf Institute}

\usepackage{blindtext}
\usepackage{comment}

\usetheme{Board}

\begin{document}

\maketitle

\block{~}
{
    \blindtext
}

\begin{columns}
    \column{0.4}
    \block{More text}{Text and more text}
    
    \column{0.6}
    \block{Something else}{Here, \blindtext \vspace{4cm}}
    \note[
        targetoffsetx=-9cm, 
        targetoffsety=-6.5cm, 
        width=0.5\linewidth
        ]
        {e-mail \texttt{welcome@overleaf.com}}
\end{columns}

\begin{columns}
    \column{0.5}
    \block{A figure}
    {
        \begin{tikzfigure}
            \includegraphics[width=0.4\textwidth]{images/overleaf-logo}
        \end{tikzfigure}
    }
    \column{0.5}
    \block{Description of the figure}{\blindtext}
\end{columns}

\end{document}

```

--------------------------------

### Change style of a single page to empty

Source: https://www.overleaf.com/learn/latex/Headers_and_footers

This example shows how to remove headers and footers from a specific page, such as a chapter page, by using `\thispagestyle{empty}` within the document body.

```latex
\begin{document}
\maketitle
\chapter{Using different page styles}
\lipsum[1] % via \usepackage{lipsum}
\chapter{Sample Chapter}
\thispagestyle{empty} % remove headers/footers from the chapter page
\lipsum[1]
\clearpage
\section{New section}
\lipsum[1]
\end{document}


```

--------------------------------

### Drawing Fermion Lines Between Vertices

Source: https://www.overleaf.com/learn/latex/Feynman_diagrams

Draws a fermion line connecting specified vertices. If a vertex name is new, it is automatically created. 'v1' is a newly created vertex in this example.

```latex
% Will create a fermion line between i1 and
% the newly created v1, and between v1 and o1.
\fmf{fermion}{i1,v1,o1}
```

--------------------------------

### Create a Numbered List with enumerate

Source: https://www.overleaf.com/learn/latex/Lists

Use the `enumerate` environment for ordered lists. The `\item` command automatically generates sequential numbers for each list entry.

```latex
Numbered (ordered) lists are easy to create:
\begin{enumerate}
  \item Items are numbered automatically.
  \item The numbers start at 1 with each use of the \texttt{enumerate} environment.
  \item Another entry in the list
\end{enumerate}

```

--------------------------------

### XeTeX Usage Overview

Source: https://www.overleaf.com/learn/latex/TeX_engine_command_line_options_for_pdfTeX%2C_XeTeX_and_LuaTeX

This is a general usage pattern for XeTeX, showing how to invoke it with different arguments. It covers running on a file, processing commands, or using a specified format file.

```text
Usage: xetex [OPTION]... [TEXNAME[.tex]] [COMMANDS]
   or: xetex [OPTION]... \FIRST-LINE
   or: xetex [OPTION]... &FMT ARGS
  Run XeTeX on TEXNAME, usually creating TEXNAME.pdf.
  Any remaining COMMANDS are processed as XeTeX input, after TEXNAME is read.
  If the first line of TEXNAME is %&FMT, and FMT is an existing .fmt file,
  use it.  Else use `NAME.fmt', where NAME is the program invocation name,
  most commonly `xetex'.

  Alternatively, if the first non-option argument begins with a backslash,
  interpret all non-option arguments as a line of XeTeX input.

  Alternatively, if the first non-option argument begins with a &, the
  next word is taken as the FMT to read, overriding all else.  Any
  remaining arguments are processed as above.

  If no arguments or options are specified, prompt for input.
```

--------------------------------

### Preliminary Declarations for a LaTeX Package

Source: https://www.overleaf.com/learn/latex/Writing_your_own_package

Import external packages using `\RequirePackage` and define custom commands or colors. `\RequirePackage` is recommended over `\usepackage` within packages.

```latex
\NeedsTeXFormat{LaTeX2e}
\ProvidesPackage{examplepackage}[2014/08/21 Example package]

\RequirePackage{imakeidx}
\RequirePackage{xstring}
\RequirePackage{xcolor}
\definecolor{greycolour}{HTML}{525252}
\definecolor{sharelatexcolour}{HTML}{882B21}
\definecolor{mybluecolour}{HTML}{394773}
\newcommand{\wordcolour}{greycolour}

```

--------------------------------

### LaTeX: Missing terminating $$ for display math

Source: https://www.overleaf.com/learn/latex/Errors%3ADisplay_math_should_end_with_%24%24.%22

This example demonstrates an error where both terminating `$` characters for display math are omitted, triggering both 'Missing $ inserted' and 'Display math should end with $$' errors.

```latex
\documentclass{article}
\usepackage[textwidth=8cm]{geometry}
\begin{document}
\noindent The following example omits both terminating \texttt{\$} characters, triggering the errors \texttt{Missing \$ inserted} and \texttt{Display math should end with \$\,.}

$$E=mc^2
\end{document}

```

--------------------------------

### Correct verbatim text with \texttt{verbatim} environment

Source: https://www.overleaf.com/learn/latex/Errors/LaTeX_Error%3A_%5Cverb_ended_by_end_of_line

This demonstrates the correct way to include verbatim text, including LaTeX commands, by using the \texttt{verbatim} environment. Load the \texttt{verbatim} package in your preamble.

```latex
% In your preamble

\usepackage{verbatim}

% In the main body of your document

We can write different typefaces in \LaTeX as
\begin{verbatim}
\textbf{Bold}
\textit{italics}
\textsf{sans serif}
\end{verbatim}
```

--------------------------------

### Define LaTeX Environment with Arguments

Source: https://www.overleaf.com/learn/latex/Environments%23Defining_a_new_environment

Define a new environment that accepts one optional and one mandatory argument. The optional argument has a default value.

```latex
\newenvironment{boxed}[2][This is a box]{\begin{center}
    Argument 1 (\#1)=#1\[1ex]
    \begin{tabular}{{{!}}p{0.9\textwidth}{{!}}}
    \hline\
    Argument 2 (\#2)=#2\[2ex]
    }
    { 
    \\\\hline
    \end{tabular} 
    \end{center}
    }
```

--------------------------------

### Special Case: Double Subscript Error with `\vec`

Source: https://www.overleaf.com/learn/latex/Errors/Double_subscript

This example shows a double subscript error that occurs even with braces when using `\vec`. It requires the `accents` package for resolution.

```latex
\documentclass{article}
\begin{document}
\({\vec a_b}_c\)
\end{document}
```

--------------------------------

### Emoji in Math Expressions with the 'emoji' Package

Source: https://www.overleaf.com/learn/latex/Articles/An_overview_of_technologies_supporting_the_use_of_colour_emoji_fonts_in_LaTeX

This LaTeX example demonstrates using the 'emoji' package to embed emoji characters within mathematical expressions, requiring unicode-math and fontspec.

```latex
\documentclass{article}
\usepackage{emoji}
\usepackage{unicode-math,fontspec}
\setmainfont{STIX}
\setmathfont{STIX Two Math}
\begin{document}
\newcommand{\emomath}[1]{\text{\emoji{#1}}}
[ 
e^{\emomath{droplet} \ln\emomath{smile}}=\emomath{sweat-smile}
]
[ 
e^{\emomath{eye}\emomath{pie}}=-1
]
\end{document}

```

--------------------------------

### Creating an Unordered List in LaTeX

Source: https://www.overleaf.com/learn/latex/Creating_a_document_in_LaTeX

Use the itemize environment to create unordered lists. Each list entry must be preceded by the \item command.

```latex
\documentclass{article}
\begin{document}
\begin{itemize}
  \item The individual entries are indicated with a black dot, a so-called bullet.
  \item The text in the entries may be of any length.
\end{itemize}
\end{document}
```

--------------------------------

### LaTeX Document with Custom Index Style

Source: https://www.overleaf.com/learn/latex/Indices

A complete LaTeX document demonstrating the use of a custom index style file (`example_style.ist`) with the `imakeidx` package. Ensure the style file is in the same directory.

```latex
\documentclass{article}
\usepackage[utf8]{inputenc}
\usepackage[T1]{fontenc}
\usepackage{imakeidx}
\makeindex[columns=3, title=Alphabetical Index, 
           options= -s example_style.ist]

\begin{document}

\tableofcontents

\section{Introduction}
In this example, several keywords\index{keywords} will be used which are important and deserve to appear in the Index\index{Index}.

Terms like generate\index{generate}, a great\index{great} list and som other\index{others} terms that might be important\index{important} 
will also show up. Terms in the index can also be nested \index{Index!nested} 

\clearpage

\section{Second section}
This second section\index{section} may include some special word, and expand the ones already used\index{keywords!used}. 

\printindex
\end{document}

```

--------------------------------

### amsmath Matrix Environments

Source: https://www.overleaf.com/learn/latex/Matrices

Use these environments from the amsmath package to typeset matrices with different delimiters. Load \usepackage{amsmath} in your preamble.

```latex
\begin{matrix}
1 & 2 & 3\\
a & b & c
\end{matrix}
```

```latex
\begin{pmatrix}
1 & 2 & 3\\
a & b & c
\end{pmatrix}
```

```latex
\begin{bmatrix}
1 & 2 & 3\\
a & b & c
\end{bmatrix}
```

```latex
\begin{Bmatrix}
1 & 2 & 3\\
a & b & c
\end{Bmatrix}
```

```latex
\begin{vmatrix}
1 & 2 & 3\\
a & b & c
\end{vmatrix}
```

```latex
\begin{Vmatrix}
1 & 2 & 3\\
a & b & c
\end{Vmatrix}
```

--------------------------------

### Multiple Integrals with amsmath and esint

Source: https://www.overleaf.com/learn/latex/Integrals%2C_sums_and_limits

Requires the `amsmath` and `esint` packages. Use commands like `\iint`, `\iiint`, `\iiiint`, `\idotsint`, and `\oint` for multiple and cyclic integrals.

```latex
\begin{gather*}
    \iint_V \mu(u,v) \,du\,dv
\
    \iiint_V \mu(u,v,w) \,du\,dv\,dw
\
    \iiiint_V \mu(t,u,v,w) \,dt\,du\,dv\,dw
\
    \idotsint_V \mu(u_1,\dots,u_k) \,du_1 \dots du_k
\end{gather*}
```

```latex
\[
    \oint_V f(s) \,ds
\]
```

--------------------------------

### Use a generated format file to typeset a document

Source: https://www.overleaf.com/learn/latex/Articles/The_two_modes_of_TeX_engines%3A_INI_mode_and_production_mode

After creating a .fmt file, use this command to instruct LuaTeX to use that specific format for typesetting your document, leading to faster compilation.

```bash
luatex --fmt=lualatex yourfile.tex
```

--------------------------------

### Creating New Paragraphs in LaTeX

Source: https://www.overleaf.com/learn/latex/Line_breaks_and_blank_spaces

A new paragraph is started by leaving an empty line in the LaTeX source code. Single line breaks within the code are treated as spaces.

```latex
\documentclass{article}
\begin{document}
This paragraph contains no information
and its purpose is to provide an example on how to start a new paragraph.
As you can see,
single line
break in the code
acts as a space in text.

However, leaving an empty line starts a new paragraph.
\end{document}

```

--------------------------------

### Incorrect Line Break at Start of Paragraph

Source: https://www.overleaf.com/learn/latex/Errors/LaTeX_Error%3A_There%27s_no_line_here_to_end

Avoid using line-breaking commands like \ or \newline when there is no preceding text to form a paragraph. This can lead to the 'no line here to end' error.

```latex
\\ We have put a line-break in the wrong place

```

--------------------------------

### Import System Font for Beamer Presentation

Source: https://www.overleaf.com/learn/latex/Beamer

Integrate a specific font family from your system into the Beamer presentation using the \usepackage command. The availability of fonts depends on your LaTeX installation.

```latex
\documentclass{beamer}
\usepackage{bookman}
\usetheme{Madrid}

```

--------------------------------

### Include and Format Code Listings with minted

Source: https://www.overleaf.com/learn/latex/Code_Highlighting_with_minted

Demonstrates how to use the minted package to include code listings from external files or directly in the LaTeX document. It shows how to add captions, labels, and generate a list of listings.

```latex
\documentclass{article}
\usepackage{minted}
\title{Listing code examples}
\begin{document}
\begin{listing}[!ht]
\inputminted{octave}{BitXorMatrix.m}
\caption{Example from external file}
\label{listing:1}
\end{document}

\begin{listing}[!ht]
\begin{minted}{c}
#include <stdio.h>
int main() {
   printf("Hello, World!"); /*printf() outputs the quoted string*/
   return 0;
}
\end{minted}
\caption{Hello World in C}
\label{listing:2}
\end{listing}

\begin{listing}[!ht]
\begin{minted}{lua}
function fact (n)--defines a factorial function
  if n == 0 then
    return 1
  else
    return n * fact(n-1)
  end
end

print("enter a number:")
a = io.read("*number") -- read a number
print(fact(a))
\end{minted}
\caption{Example from the Lua manual}
\label{listing:3}
\end{listing}
\noindent\texttt{minted} makes a nice job of typesetting listings \ref{listing:1}, \ref{listing:2} and \ref{listing:3}.
\renewcommand\listoflistingscaption{List of source codes}
\listoflistings
\end{document}
```

--------------------------------

### Default Operator Spacing in Math Mode

Source: https://www.overleaf.com/learn/latex/Spacing_in_math_mode

Shows the default spacing around relational and binary operators in LaTeX math mode. This example highlights the subtle differences between default spacings.

```latex
\begin{align*}
3ax+4by=5cz\\
3ax<4by+5cz
\end{align*}
```

--------------------------------

### Specify Font for Devanagari Script

Source: https://www.overleaf.com/learn/latex/Multilingual_typesetting_on_Overleaf_using_babel_and_fontspec

This example shows how to set a specific font for the Devanagari script, used for Hindi and Sanskrit. It uses babel for language import and fontspec for font selection.

```latex
\documentclass[12pt]{article}
\usepackage[english]{babel}
%% Each \babelprovide can only be used for one language
\babelprovide[import]{hindi}
\babelprovide[import]{sanskrit}
\babelfont[*devanagari]{rm}{Lohit Devanagari}
\begin{document}
Hindi: \foreignlanguage{hindi}{हिन्दी}

Sanskrit: \foreignlanguage{sanskrit}{संस्कृतम्}
\end{document}
```

--------------------------------

### Customize List Titles in LaTeX

Source: https://www.overleaf.com/learn/latex/Lists_of_tables_and_figures

Change the default titles for lists of figures and tables using \renewcommand. For example, \renewcommand{\listfigurename}{List of plots} changes the figure list title.

```latex
\documentclass{article}
\usepackage{graphicx}
\usepackage{array}
\graphicspath{ {figures/} }

\renewcommand{\listfigurename}{List of plots}
\renewcommand{\listtablename}{Tables}

\begin{document}

\thispagestyle{empty}
\listoffigures
\listoftables
\clearpage
\pagenumbering{arabic}

Lorem ipsum dolor sit amet, consectetuer adipiscing elit.  Etiam lobortisfacilisis...
\end{document}
```

--------------------------------

### Basic Listings Environment in LaTeX

Source: https://www.overleaf.com/learn/latex/Code_listing%23Code_styles_and_colours

The `lstlisting` environment from the `listings` package displays code, preserving whitespace and line breaks, similar to `verbatim` but with potential for more features.

```latex
\begin{lstlisting}
import numpy as np
    
def incmatrix(genl1,genl2):
    m = len(genl1)
    n = len(genl2)
    M = None #to become the incidence matrix
    VT = np.zeros((n*m,1), int)  #dummy variable
    
    #compute the bitwise xor matrix
    M1 = bitxormatrix(genl1)
    M2 = np.triu(bitxormatrix(genl2),1) 

    for i in range(m-1):
        for j in range(i+1, m):
            [r,c] = np.where(M2 == M1[i,j])
            for k in range(len(r)):
                VT[(i)*n + r[k]] = 1;
                VT[(i)*n + c[k]] = 1;
                VT[(j)*n + r[k]] = 1;
                VT[(j)*n + c[k]] = 1;
                
                if M is None:
                    M = np.copy(VT)
                else:
                    M = np.concatenate((M, VT), 1)
                
                VT = np.zeros((n*m,1), int)
    
    return M
\end{lstlisting}
```

--------------------------------

### URL with Underscores (using url package)

Source: https://www.overleaf.com/learn/latex/Errors/Missing_%24_inserted

When typesetting URLs with underscores, it is recommended to use the `url` or `hyperref` package and the `\url` command to handle them correctly.

```latex
\documentclass{article}
\usepackage{url} 
\begin{document}
\url{https://www.overleaf.com/learn/latex/Subscripts_and_superscripts}
\end{document}
```

--------------------------------

### Lua Long Bracket String Example

Source: https://www.overleaf.com/learn/latex/Articles/An_Introduction_to_LuaTeX_%28Part_2%29%3A_Understanding_%5Cdirectlua

Shows a string defined using Lua's long bracket syntax, where escape sequences like \n are treated as literal characters.

```string
[[I am a long brackets string]]
```

```string
[[I am a long brackets\n string]]
```

--------------------------------

### LaTeX Table with Fixed Total Width using `tabularx` package

Source: https://www.overleaf.com/learn/latex/Tables%23Colouring_a_table_.28cells.2C_rows.2C_columns_and_lines.29

Creates a table with a specified total width (e.g., `0.8\textwidth`) and distributes content across columns using the `tabularx` package. Use `X` columns for auto-width distribution and `>{\...}` for column-specific alignment.

```latex
\documentclass{article}
\usepackage{tabularx}
\begin{document}
\begin{tabularx}{0.8\textwidth} { 
  | >{\raggedright\arraybackslash}X 
  | >{\centering\arraybackslash}X 
  | >{\raggedleft\arraybackslash}X | }
 \hline
 item 11 & item 12 & item 13 \ 
 \hline
 item 21  & item 22  & item 23  \ 
\hline
\end{tabularx}
\end{document}
```

--------------------------------

### Package Identification Commands

Source: https://www.overleaf.com/learn/latex/Writing_your_own_package

Use `\NeedsTeXFormat` to specify the LaTeX version and `\ProvidesPackage` to identify the package with its name and release date.

```latex
\NeedsTeXFormat{LaTeX2e}
\ProvidesPackage{examplepackage}[2014/08/24 Example LaTeX package]

```

--------------------------------

### LaTeX Example Triggering 'Missing $ inserted' Error

Source: https://www.overleaf.com/learn/latex/Errors/Missing_%24_inserted

This LaTeX code snippet demonstrates a scenario that triggers the 'Missing $ inserted' error. It highlights the issue of incorrect math mode handling.

```latex
Writing \verb|$y=f(x)$$\vskip3pt| produces...$y=f(x)$$\vskip3pt Start new line...
```

--------------------------------

### Using Limits Operator in LaTeX with amsmath

Source: https://www.overleaf.com/learn/latex/Operators

Shows how to typeset limits using the \lim operator, which requires the `amsmath` package. It also illustrates how limits can be used with text and include subscripts.

```latex
\documentclass{article}
\usepackage{amsmath}
\begin{document}
Testing notation for limits
\[
    \lim_{h \to 0 } \frac{f(x+h)-f(x)}{h}
.\]
This operator changes when used alongside 
text \( \lim_{h \to 0} (x-h) \).
\end{document}
```

--------------------------------

### Incorrect Use of $ in amsmath align Environment

Source: https://www.overleaf.com/learn/latex/Errors/Missing_%24_inserted

This example demonstrates the 'Missing $ inserted' error by incorrectly using '$' signs within an 'align*' environment. Math content within 'align*' should not be enclosed in '$'.

```latex
\documentclass{article}
\usepackage{amsmath}
\begin{document}
\begin{align*}
$2x - 5y &=  8 \ 
3x + 9y &=  -12$
\end{align*}
\end{document}
```

--------------------------------

### Loading the amsmath Package

Source: https://www.overleaf.com/learn/latex/Fractions_and_Binomials

Shows how to include the amsmath package in the LaTeX document preamble to enable advanced mathematical typesetting features.

```latex
\usepackage{amsmath}
```

--------------------------------

### Subfile structure for `subfiles` package

Source: https://www.overleaf.com/learn/latex/Multi-file_LaTeX_projects

Use `\documentclass[../main.tex]{subfiles}` to reference the main document and `\subfix{../images/}` for graphics paths. Content outside `\begin{document}` and `\end{document}` is ignored.

```latex
\documentclass[../main.tex]{subfiles}
\graphicspath{{\subfix{../images/}}}
\begin{document}
\textbf{Hello world!}
\begin{figure}[bh]
\centering
\includegraphics[width=3cm]{overleaf-logo}

\label{fig:img1}
\caption{Overleaf logo}
\end{figure}

Hello, here is some text without a meaning...

\end{document}
```

--------------------------------

### Fixing tabular error by adding an extra column

Source: https://www.overleaf.com/learn/latex/Errors/Extra_alignment_tab_has_been_changed_to_%5Ccr

This example demonstrates fixing the 'Extra alignment tab' error by adjusting the \begin{tabular} environment definition to accommodate the extra column.

```latex
\begin{center}
\begin{tabular}{c|c|c|c}
   1 & 2 & 3 & 4\ 
   5 & 6 & 7\ 
\end{tabular}
\end{center}
```

--------------------------------

### Use Custom LaTeX Class

Source: https://www.overleaf.com/learn/latex/Writing_your_own_class

Demonstrates how to use a custom LaTeX class in a document. The class is specified in \documentclass, and options can be passed in square brackets.

```tex
\documentclass[red]{exampleclass}
\usepackage[utf8]{inputenc}
\usepackage[english]{babel}

\usepackage{blindtext}

\title{Example to show how classes work}
\author{Team Learn ShareLaTeX}
\date{August 2014}

\begin{document}

\maketitle

\noindent
Let's begin with a simple working example here.

\blindtext

\section{Introduction}

The Monty Hall problem...

\section{The same thing}

The Monty...


```

--------------------------------

### BibTeX Author Formatting: Incorrect Examples

Source: https://www.overleaf.com/learn/latex/Bibliography_management_with_bibtex

Avoid these incorrect author formats in BibTeX, as they can lead to errors or unexpected output. Commas should only separate last and first names, not individual authors.

```bibtex
author = {Jand Doe, John Goodneough, Foo Bar}

```

```bibtex
author = {Jand Doe, John Goodneough and Foo Bar}

```

```bibtex
author = {Jand Doe, John Goodneough, and Foo Bar}


```

--------------------------------

### Generate MLTeX and EncTeX Format File using -etex

Source: https://www.overleaf.com/learn/latex/MLTeX_EncTeX_and_SyncTeX_TeX_extensions

Alternatively, use the -etex command-line option instead of an asterisk to enable extended mode for generating the format file.

```bash
pdftex -ini -etex -enc -mltex pdfetex.ini
```

--------------------------------

### Import minted Package

Source: https://www.overleaf.com/learn/latex/Code_Highlighting_with_minted

Include this line in your LaTeX preamble to enable the minted package for code highlighting.

```latex
\usepackage{minted}
```

--------------------------------

### LuaHBTeX Syntax Error Example

Source: https://www.overleaf.com/learn/latex/Articles/An_overview_of_technologies_supporting_the_use_of_colour_emoji_fonts_in_LaTeX

This output demonstrates the typical Lua syntax error encountered when \char commands are improperly used within \directlua, indicating an invalid escape sequence.

```text
[\directlua]:1: invalid escape sequence near '"\c'.
\codestoemoji ...ing \includegraphics }.]]) end }
                                                  
l.75 ...r"E0065\Uchar"E006E\Uchar"E0067\Uchar"E007F}
                                                  
The lua interpreter ran into a problem, so the
remainder of this lua chunk will be ignored.

```

--------------------------------

### Call Macro with \Uchar Commands

Source: https://www.overleaf.com/learn/latex/Articles/An_overview_of_technologies_supporting_the_use_of_colour_emoji_fonts_in_LaTeX

Example of calling a LaTeX macro with \Uchar commands, which are used to specify Unicode character codes. This input format is transformed into UTF-8 text for HarfBuzz.

```latex
\codestoemoji{\Uchar"1F3F4\Uchar"E0067\Uchar"E0062\Uchar"E0065\Uchar"E006E\Uchar"E0067\Uchar"E007F}

```

--------------------------------

### Customized Python Code Highlighting

Source: https://www.overleaf.com/learn/latex/Code_Highlighting_with_minted

Configure the minted environment with options like frame, background color, font size, and line numbers for enhanced code presentation. Ensure xcolor is imported for color options.

```latex
\documentclass{article}
\usepackage{minted}
\usepackage{xcolor} % to access the named colour LightGray
\definecolor{LightGray}{gray}{0.9}
\begin{document}
\begin{minted}
[ 
frame=lines,
framesep=2mm,
baselinestretch=1.2,
bgcolor=LightGray,
fontsize=\footnotesize,
linenos
]
{python}
import numpy as np
    
def incmatrix(genl1,genl2):
    m = len(genl1)
    n = len(genl2)
    M = None #to become the incidence matrix
    VT = np.zeros((n*m,1), int)  #dummy variable
    
    #compute the bitwise xor matrix
    M1 = bitxormatrix(genl1)
    M2 = np.triu(bitxormatrix(genl2),1) 

    for i in range(m-1):
        for j in range(i+1, m):
            [r,c] = np.where(M2 == M1[i,j])
            for k in range(len(r)):
                VT[(i)*n + r[k]] = 1;
                VT[(i)*n + c[k]] = 1;
                VT[(j)*n + r[k]] = 1;
                VT[(j)*n + c[k]] = 1;
                
                if M is None:
                    M = np.copy(VT)
                else:
                    M = np.concatenate((M, VT), 1)
                
                VT = np.zeros((n*m,1), int)
    
    return M
\end{minted}
\end{document}
```

--------------------------------

### Plain TeX (XeTeX) Fails to Load Raster-Based Color Emoji Font

Source: https://www.overleaf.com/learn/latex/Articles/An_overview_of_technologies_supporting_the_use_of_colour_emoji_fonts_in_LaTeX

A Plain TeX example using XeTeX to load NotoColorEmoji.ttf also fails, reporting that the font is not loadable.

```tex
\font\emojifont="[NotoColorEmoji.ttf]" at 12pt
\emojifont \char"1F600
\bye

```

--------------------------------

### Activate MLTeX with pdfTeX

Source: https://www.overleaf.com/learn/latex/MLTeX_EncTeX_and_SyncTeX_TeX_extensions

Use this command to create a format file with MLTeX enabled. The asterisk enables extended mode for e-TeX primitives.

```bash
pdftex -ini -mltex *pdfetex.ini

```

--------------------------------

### Apply Smallest Font Size and Small Caps Style in LaTeX

Source: https://www.overleaf.com/learn/latex/Font_sizes_and_kinds%23Reference_guide

Demonstrates how to use the \tiny command for the smallest font size and \textsc{...} for small caps style within a text block.

```latex
This is a simple example, {\tiny this will show different font sizes} and also \textsc{different font styles}.
```

--------------------------------

### Define Macros with and without \protected

Source: https://www.overleaf.com/learn/latex/Articles/An_Introduction_to_LuaTeX_%28Part_2%29%3A_Understanding_%5Cdirectlua

Demonstrates defining macros with and without the \protected prefix. Use \protected for macros that should not be expanded during certain TeX operations.

```tex
\def\macroA{"This unprotected macro contains a string"}
\protected\def\macroB{"This protected macro also contains a string"}
```

--------------------------------

### Setting Paragraph Indentation in LaTeX

Source: https://www.overleaf.com/learn/latex/Paragraphs_and_new_lines

Shows how to set the paragraph indentation amount using \setlength{\parindent}{value}. A value of 0pt or using \noindent at the start of a paragraph will prevent indentation.

```latex
\setlength{\parindent}{20pt}

```

--------------------------------

### Set Page Background Color in LaTeX

Source: https://www.overleaf.com/learn/latex/Using_colours_in_LaTeX

Use \pagecolor to set the background color for the entire document. \nopagecolor can be used to revert to the default. This example also demonstrates text and item coloring.

```latex
\documentclass{article}
\usepackage[dvipsnames]{xcolor}
\colorlet{LightRubineRed}{RubineRed!70}
\colorlet{Mycolor1}{green!10!orange}
\definecolor{Mycolor2}{HTML}{00F9DE}
\begin{document}
\pagecolor{black}
\color{white}% set the default colour to white
This document presents several examples showing how to use the \texttt{xcolor} package 
to change the color of \LaTeX{} page elements.

\begin{itemize}
\item \textcolor{Mycolor1}{First item}
\item \textcolor{Mycolor2}{Second item}
\end{itemize}

\noindent
{\color{LightRubineRed} \rule{\linewidth}{1mm}}

\noindent
{\color{RubineRed} \rule{\linewidth}{1mm}}
\end{document}


```

--------------------------------

### Consecutive \expandafter Commands

Source: https://www.overleaf.com/learn/latex/Articles/How_does_%5Cexpandafter_work%3A_A_detailed_study_of_consecutive_%5Cexpandafter_commands

Illustrates the basic structure of multiple consecutive \expandafter commands.

```latex
\expandafter\expandafter\expandafter...
```

--------------------------------

### Define Sans Serif Font for Hebrew Sections

Source: https://www.overleaf.com/learn/latex/Multilingual_typesetting_on_Overleaf_using_babel_and_fontspec

This example demonstrates defining a specific sans-serif font for Hebrew. It also customizes section titles to use the sans-serif font and bold formatting.

```latex
\documentclass[12pt]{article}
\usepackage{geometry} % to use a small page size
\geometry{margin=4cm,b5paper}
\usepackage[bidi=basic]{babel}
\babelprovide[main,import]{hebrew}
\babelfont[hebrew]{rm}{Hadasim CLM}
\babelfont[hebrew]{sf}{Miriam CLM}
\usepackage{titlesec}
\titleformat{\section}{\Large\sffamily\bfseries}{\thesection}{1em}{}
\begin{document}
\section{מבוא}

זוהי עובדה מבוססת שדעתו של הקורא תהיה מוסחת עלידי טקטס קריא כאשר הוא יביט בפריסתו.
\end{document}
```

--------------------------------

### Horizontal Glue Syntax with \hskip

Source: https://www.overleaf.com/learn/latex/Articles/How_TeX_Calculates_Glue_Settings_in_an_%5Chbox

Shows the general syntax for defining horizontal glue using \hskip, specifying natural width, stretch amount (plus), and shrink amount (minus).

```tex
\hskip 3pt plus 2pt minus 1pt
```

--------------------------------

### Double-sided Document Headers and Footers

Source: https://www.overleaf.com/learn/latex/Headers_and_footers

Configure headers and footers for double-sided documents using fancyhdr. This example sets specific text for odd/even right/left headers and footers, including page numbers.

```latex
\documentclass{book}
\usepackage[paperheight=16cm, paperwidth=12cm,% Set the height and width of the paper
includehead,
nomarginpar,% We don't want any margin paragraphs
textwidth=10cm,% Set \textwidth to 10cm
headheight=10mm,% Set \headheight to 10mm
]{geometry}
\usepackage{fancyhdr}
\begin{document}
% Set the page style to "fancy"...
\pagestyle{fancy}
%... then configure it.
\fancyhead{} % clear all header fields
\fancyhead[RO,LE]{\textbf{The performance of new graduates}}
\fancyfoot{} % clear all footer fields
\fancyfoot[LE,RO]{\thepage}
\fancyfoot[LO,CE]{From: K. Grant}
\fancyfoot[CO,RE]{To: Dean A. Smith}
% Some content:
This is page 1.\newpage
This is page 2.
\end{document}
```

--------------------------------

### Enable EncTeX with pdfTeX

Source: https://www.overleaf.com/learn/latex/MLTeX_EncTeX_and_SyncTeX_TeX_extensions

Activates EncTeX by creating an EncTeX-enabled format file using pdfTeX in INI mode with the -enc option.

```bash
pdftex -ini -enc *pdfetex.ini

```

--------------------------------

### Using \counterwithout to Reset Counter Independence

Source: https://www.overleaf.com/learn/latex/Counters%23Accessing_and_printing_counter_values

Illustrates how \counterwithout removes the dependency between two counters, allowing one to increment independently of the other. This is shown by making the 'example' counter independent of the 'section' counter.

```latex
\section{First equation}
\begin{example}
\begin{equation}
    f(x)=\frac{x}{1+x^2}
\end{equation}
\end{example}

\subsection{Second equation}
\begin{example}
\begin{equation}
    f(x)=\frac{x+1}{x-1}
\end{equation}
\end{example}
\vspace{6pt}\noindent Note how the \texttt{example} counter is reset at the start Section \ref{sec:n0}. 
\section{Third equation}
\label{sec:n0}
\begin{example}
\begin{equation}
    f(x)=\frac{x+1}{x-1}
\end{equation}
\end{example}
\vspace{6pt}\noindent Here, we wrote \verb|\counterwithout{example}{section}| so that \texttt{example} is no longer reset at the start of a section. In Sections \ref{sec:n1} and \ref{sec:n2} the \texttt{example} counter keeps increasing. \counterwithout{example}{section}
\section{Fourth equation}
\label{sec:n1}
\begin{example}
\begin{equation}
    f(x)=\frac{x^2+x^3}{1+x^3}
\end{equation}
\end{example}
\section{Fifth equation}
\label{sec:n2}
\begin{example}
\begin{equation}
    f(x,k)=\frac{x^2-x^k}{1+x^3}
\end{equation}
\end{example}

```

--------------------------------

### LaTeX: Duplicate Labels for Section and Image

Source: https://www.overleaf.com/learn/latex/Errors/There_were_multiply-defined_labels

This example shows a common cause of the 'multiply-defined labels' error where the same label is applied to both a section and an image. Ensure each label is unique to avoid referencing confusion.

```latex
\section{Section 1}
\label{algebra}

\includegraphics{image.jpg}
\label{algebra}

```

--------------------------------

### Basic Image Insertion with Caption

Source: https://www.overleaf.com/learn/latex/Inserting_Images%23Positioning

Use the `figure` environment and `egin{figure}` to include an image with a caption. The `	extwidth` unit scales the image relative to the text width.

```latex
\begin{figure}[h]
\caption{Example of a parametric plot ($\sin (x), \cos(x), x$)}
\centering
\includegraphics[width=0.5\textwidth]{spiral}
\end{figure}
```

--------------------------------

### Fixing tabular error by removing extra column entry

Source: https://www.overleaf.com/learn/latex/Errors/Extra_alignment_tab_has_been_changed_to_%5Ccr

This example shows how to fix the 'Extra alignment tab' error in a \begin{tabular} environment by removing the erroneous column entry from the row.

```latex
\begin{center}
\begin{tabular}{c|c|c}
   1 & 2 & 3\ 
   5 & 6 & 7\ 
\end{tabular}
\end{center}
```

--------------------------------

### Expanded Token List for \hello Macro

Source: https://www.overleaf.com/learn/latex/Articles/%5Cexpandafter_TeX_tokens?preview=true

Shows the full sequence of token values that TeX generates when the \hello macro is fully expanded. This list includes characters, spaces, and commands like \TeX and \hskip, with their corresponding integer token values and human-readable representations.

```plaintext
2887 (G), 2930 (r), 2917 (e), 2917 (e), 2932 (t), 2921 (i), 2926 (n), 2919 (g), 2931 (s), 3116 (,), 2592 (<space>), 2918 (f), 2930 (r), 2927 (o), 2925 (m), 2592 (<space>),  2900 (T), 19598 (\kern), 3117 (-), 3118 (.), 3121 (1), 3126 (6), 3126 (6), 3127 (7), 2917 (e), 2925 (m), 19597 (\lower), 3118 (.), 3125 (5), 2917 (e), 2936 (x), 6175 (\hbox), 379 ({), 2885 (E), 637 (}), 19598 (\kern), 3117 (-), 3118 (.), 3121 (1), 3122 (2), 3125 (5), 2917 (e), 2925 (m), 2904 (X), 3118 (.), 2592 (<space>), 7943 (\hskip), 3121 (1), 3120 (0), 2928 (p), 2932 (t)
```

--------------------------------

### Basic Lstlisting Environment in LaTeX

Source: https://www.overleaf.com/learn/latex/Code_listing

The `lstlisting` environment from the `listings` package displays code, preserving whitespace and line breaks, and ignoring LaTeX commands. This is useful for general code display.

```latex
\begin{lstlisting}
import numpy as np
    
def incmatrix(genl1,genl2):
    m = len(genl1)
    n = len(genl2)
    M = None #to become the incidence matrix
    VT = np.zeros((n*m,1), int)  #dummy variable
    
    #compute the bitwise xor matrix
    M1 = bitxormormatrix(genl1)
    M2 = np.triu(bitxormatrix(genl2),1) 

    for i in range(m-1):
        for j in range(i+1, m):
            [r,c] = np.where(M2 == M1[i,j])
            for k in range(len(r)):
                VT[(i)*n + r[k]] = 1;
                VT[(i)*n + c[k]] = 1;
                VT[(j)*n + r[k]] = 1;
                VT[(j)*n + c[k]] = 1;
                
                if M is None:
                    M = np.copy(VT)
                else:
                    M = np.concatenate((M, VT), 1)
                
                VT = np.zeros((n*m,1), int)
    
    return M
\end{lstlisting}
```

--------------------------------

### Create and Modify LaTeX Counters

Source: https://www.overleaf.com/learn/latex/Counters%23Introduction_to_LaTeX_counters

Demonstrates creating a counter, setting its initial value, and then incrementing it using \addtocounter. The \the command is used to display the counter's value.

```latex
\newcounter{myvar}
\setcounter{myvar}{42} 

\noindent Writing \verb|\themymar| typesets \texttt{\themyvar}.

\noindent Next, we’ll change \texttt{myvar} to \texttt{142} by writing \verb|\addtocounter{myvar}{100}|\addtocounter{myvar}{100}. Now, writing \verb|\themyvar| outputs \texttt{\themyvar}.
```

--------------------------------

### Set BibTeX Style and Import Bibliography

Source: https://www.overleaf.com/learn/latex/Bibliography_styles%23Natbib_styles

Use \bibliographystyle to set the bibliography style and \bibliography to import the .bib file. Ensure the .bib file name is provided without the extension.

```latex
\bibliographystyle{stylename}
  \bibliography{bibfile}
```

--------------------------------

### Drawing Arrows with the vector Command

Source: https://www.overleaf.com/learn/latex/Picture_environment

Illustrates the use of the \vector command to draw arrows within a picture environment. It shares syntax with the \line command, specifying start point, direction, and length.

```latex
\documentclass{article}
\usepackage[pdftex]{pict2e}
\begin{document}
\setlength{\unitlength}{0.20mm}
\begin{picture}(400,250)
\put(75,10){\line(1,0){130}}
\put(75,50){\line(1,0){130}}
\put(75,200){\line(1,0){130}}
\put(120,200){\vector(0,-1){150}}
\put(190,200){\vector(0,-1){190}}
\put(97,120){$\alpha$}
\put(170,120){$\beta$}
\put(220,195){upper state}
\put(220,45){lower state 1}
\put(220,5){lower state 2}
\end{picture}
\end{document}
```

--------------------------------

### Linking Web Addresses and Local Files in LaTeX

Source: https://www.overleaf.com/learn/latex/Hyperlinks

Create hyperlinks to external websites using \texttt{\href} or \texttt{\url}, and link to local files using \texttt{\href} with the \texttt{run:} prefix. \texttt{\href} allows for custom link text, while \texttt{\url} displays the URL itself.

```latex
For further references see \href{http://www.overleaf.com}{Something Linky} 
or go to the next url: \url{http://www.overleaf.com}
```

--------------------------------

### Modifying Enumerate List Counter Values

Source: https://www.overleaf.com/learn/latex/Counters%23Accessing_and_printing_counter_values

Shows how to change the starting number of an enumerate list by using the \setcounter command on the 'enumi' counter. This allows lists to begin at a specific number, such as 4.

```latex
This example shows one way to change the numbering of a list; here, changing the value of the \texttt{enumi} counter to start the list numbering at 4 (it is incremented by the \verb|\item| command):

\begin{enumerate}
\setcounter{enumi}{3}
\item Something.
\item Something else.
\item Another element.
\item The last item in the list.
\end{enumerate}

```

--------------------------------

### Resolving LaTeX Float Specifier Error: Inserting Specifier

Source: https://www.overleaf.com/learn/latex/Errors%3ANo_positions_in_optional_float_specifier.%22

Alternatively, resolve the error by inserting a valid float specifier within the square brackets to guide LaTeX's placement of the float.

```latex
\begin{figure}[h]
```

```latex
\begin{table}[!htb]
```

--------------------------------

### TEXMF Configuration Variable

Source: https://www.overleaf.com/learn/latex/Articles/An_introduction_to_Kpathsea_and_how_TeX_engines_search_for_files

Defines the TEXMF variable, which specifies a series of top-level folders (texmf trees) for searching TeX-related files. This is useful for splitting TeX installations across multiple directories.

```tex
TEXMF = {$TEXMFAUXTREES$TEXMFCONFIG,$TEXMFVAR,$TEXMFHOME,!!$TEXMFLOCAL,!!$TEXMFSYSCONFIG,!!$TEXMFSYSVAR,!!$TEXMFDIST}
```

--------------------------------

### LaTeX Table Environment with '!htb' Specifiers

Source: https://www.overleaf.com/learn/latex/Errors/No_positions_in_optional_float_specifier

This code demonstrates the correct usage of the \begin{table} environment with multiple float specifiers ('!', 'h', 't', 'b'). This provides LaTeX with placement options and avoids the 'No positions in optional float specifier' error.

```latex
\begin{table}[!htb]
  % Table content here
\end{table}
```

--------------------------------

### Direct Macro Call vs. \expandafter

Source: https://www.overleaf.com/learn/latex/Articles/How_does_%5Cexpandafter_work%3A_A_detailed_study_of_consecutive_%5Cexpandafter_commands

Compares the output of calling \foo\bar directly versus using \expandafter\foo\bar. This highlights the difference in argument absorption and expansion.

```latex
\foo\bar
```

```latex
\expandafter\foo\bar
```

--------------------------------

### LaTeX Report Document Class Example

Source: https://www.overleaf.com/learn/latex/Sections_and_chapters

A basic LaTeX document using the `report` class, demonstrating the structure with title, table of contents, chapters, and sections. This class supports standard sectioning commands.

```latex
\documentclass{report}
\title{Sections and Chapters}
\author{Overleaf}
\date{\today}
\begin{document}
\maketitle
\tableofcontents
\chapter{An Introduction to Lua\TeX}

\section{What is it—and what makes it so different?}
Lua\TeX{} is a \textit{toolkit}—it contains sophisticated software tools and components with which you can construct (typeset) a wide range of documents. The sub-title of this article also poses two questions about Lua\TeX: What is it—and what makes it so different? The answer to “What is it?” may seem obvious: “It’s a \TeX{} typesetting engine!” Indeed it is, but a broader view, and one to which this author subscribes, is that Lua\TeX{} is an extremely versatile \TeX-based document construction and engineering system.

\subsection{Explaining Lua\TeX: Where to start?}
The goal of this first article on Lua\TeX{} is to offer a context for understanding what this TeX engine provides and why/how its design enables users to build/design/create a wide range of solutions to complex typesetting and design problems—perhaps also offering some degree of “future proofing” 

\chapter{Lua\TeX: Background and history}
\section{Introduction}
Lua\TeX{} is, in \TeX{} terms, “the new kid on the block” despite having been in active development for over 10 years.

\subsection{Lua\TeX: Opening up \TeX’s “black box”}
Knuth’s original \TeX{} program is the common ancestor of all modern \TeX{} engines in use today and Lua\TeX{} is, in effect, the latest evolutionary step: derived from the pdf\TeX{} program but with the addition of some powerful software components which bring a great deal of extra functionality.
\end{document}
```

--------------------------------

### Typeset Korean with xeCJK and Project Fonts

Source: https://www.overleaf.com/learn/latex/Korean

Utilize the xeCJK package with XeLaTeX to typeset Korean text using custom fonts uploaded directly into your Overleaf project. This example uses 'UnGungseo.ttf' and 'gulim.ttf'.

```latex
\documentclass{article}
\usepackage{xeCJK}

\setmainfont{Noto Serif}
\setCJKmainfont{UnGungseo.ttf}
\setCJKsansfont{UnGungseo.ttf}
\setCJKmonofont{gulim.ttf}

\begin{document}

\section{소개}
전체 문서에 대한 기본 정보를 소개 단락.

\begin{verbatim}
그것은 간격 방법을 참조 그대로 글꼴을 테스트
\end{verbatim}

Latin characters are also allowed.

\end{document}

```

--------------------------------

### Apply Stylesheet for Code Highlighting

Source: https://www.overleaf.com/learn/latex/Code_Highlighting_with_minted

Customize code highlighting appearance by using \usemintedstyle followed by the desired stylesheet name (e.g., 'borland'). This command should be placed before the \begin{document} environment.

```latex
\documentclass{article}
\usepackage{minted}
\usemintedstyle{borland}
\begin{document}
\begin{minted}{python}
import numpy as np
    
def incmatrix(genl1,genl2):
    m = len(genl1)
    n = len(genl2)
    M = None #to become the incidence matrix
    VT = np.zeros((n*m,1), int)  #dummy variable
    
    #compute the bitwise xor matrix
    M1 = bitxormatrix(genl1)
    M2 = np.triu(bitxormatrix(genl2),1) 

    for i in range(m-1):
        for j in range(i+1, m):
            [r,c] = np.where(M2 == M1[i,j])
            for k in range(len(r)):
                VT[(i)*n + r[k]] = 1;
                VT[(i)*n + c[k]] = 1;
                VT[(j)*n + r[k]] = 1;
                VT[(j)*n + c[k]] = 1;
                
                if M is None:
                    M = np.copy(VT)
                else:
                    M = np.concatenate((M, VT), 1)
                
                VT = np.zeros((n*m,1), int)
    
    return M
\end{minted}
\end{document}
```

--------------------------------

### Include Glossary in Table of Contents

Source: https://www.overleaf.com/learn/latex/Glossaries

To include the glossary in the table of contents, use the \usepackage[toc]{glossaries} option in the preamble. This example shows how to add a \tableofcontents command and print the glossary with specific titles.

```latex
\documentclass{article}
\usepackage[toc]{glossaries}

\makeglossaries

\newglossaryentry{maths}
{
    name=mathematics,
    description={Mathematics is what mathematicians do}
}

\newglossaryentry{latex}
{
    name=latex,
    description={Is a markup language specially suited for 
scientific documents}
}


\newglossaryentry{formula}
{
    name=formula,
    description={A mathematical expression}
}

\begin{document}

\tableofcontents

\section{First Section} 
The \Gls{latex} typesetting markup language is specially suitable 
for documents that include \gls{maths}. \Glspl{formula} are rendered 
properly an easily once one gets used to the commands.

\clearpage

\printglossary[title=Special Terms, toctitle=List of terms]

\end{document}
```

--------------------------------

### Basic Verbatim Environment in LaTeX

Source: https://www.overleaf.com/learn/latex/Code_listing

Use the `verbatim` environment to display code exactly as typed, preserving whitespace and line breaks. All LaTeX commands within this environment are ignored.

```latex
\begin{verbatim}
Text enclosed inside \texttt{verbatim} environment 
is printed directly 
and all \LaTeX{} commands are ignored.
\end{verbatim}
```

--------------------------------

### Blank Line in Equation (Error Example)

Source: https://www.overleaf.com/learn/latex/Errors/Missing_%24_inserted

A blank line within a math environment like `equation` or between display math lines is converted to a `\par` command, which is not allowed in math mode, causing an error.

```latex
\documentclass{article}
\begin{document}
\begin{equation}
y=x^3,

z=x^3
\end{equation}
\end{document}
```

```latex
\documentclass{article}
\begin{document}
\[y=x^3,\par z=x^3\]
\end{document}
```

--------------------------------

### Define Package Options and Colors

Source: https://www.overleaf.com/learn/latex/Writing_your_own_package

This snippet shows how to define package options (red, blue) that change a color variable. It also defines custom colors and a default color. Use this to allow users to customize package behavior.

```tex
\NeedsTeXFormat{LaTeX2e}
\ProvidesPackage{examplepackage}[2014/08/21 Example package]

\RequirePackage{imakeidx}
\RequirePackage{xstring}
\RequirePackage{xcolor}
\definecolor{greycolour}{HTML}{525252}
\definecolor{sharelatexcolour}{HTML}{882B21}
\definecolor{mybluecolour}{HTML}{394773}
\newcommand{\wordcolour}{greycolour}

\DeclareOption{red}{\renewcommand{\wordcolour}{sharelatexcolour}}
\DeclareOption{blue}{\renewcommand{\wordcolour}{mybluecolour}}
\DeclareOption*{\PackageWarning{examplepackage}{Unknown ‘\CurrentOption’}}
\ProcessOptions\relax
```

--------------------------------

### Typeset an epigraph with epigraph package

Source: https://www.overleaf.com/learn/latex/Typesetting_quotations

Use the \epigraph command to typeset a quotation and its source. The first argument is the quote, and the second is the source.

```latex
\documentclass{book}
\usepackage{blindtext} %This package generates automatic text
\usepackage{epigraph} 

\title{Epigraph example}
\author{Overleaf}
\date{August 2021}

\begin{document}
\frontmatter
\mainmatter

\chapter{Something}
\epigraph{All human things are subject to decay, and when fate summons, Monarchs must obey}{\textit{Mac Flecknoe \ John Dryden}}
\blindtext
\end{document}
```

--------------------------------

### Redefining the Itemize Environment

Source: https://www.overleaf.com/learn/latex/Environments%23Defining_a_new_environment

This example redefines the standard \texttt{itemize} environment to center and italicize its content instead of creating a bulleted list. Use this to understand environment redefinition, not for practical document creation.

```latex
\documentclass{article}
% Redefine the environment in the preamble
\renewenvironment{itemize}
{\begin{center}\em}
{\end{center}}
\begin{document}

\begin{itemize}
We have redefined the \texttt{itemize} environment so that any text 
within it is centred and emphasised (italicized). It no longer creates
a bulleted list---this is only an example and not intended for use 
in real documents!
\end{itemize}
\end{document}
```

--------------------------------

### Basic Quotation with dirtytalk Package

Source: https://www.overleaf.com/learn/latex/Typesetting_quotations

Demonstrates the basic usage of the `dirtytalk` package for typesetting simple and nested quotations using the `\say` command.

```latex
\documentclass{article}
\usepackage{dirtytalk}
\begin{document}
\section{Introduction}

Typing quotations with this package is quite easy:

\say{Here, a quotation is written and even some \say{nested} quotations 
are possible}
\end{document}
```

--------------------------------

### Set short skip values and display equation

Source: https://www.overleaf.com/learn/latex/%5Cabovedisplayskip_and_related_commands

Demonstrates the effect of \\abovedisplayshortskip and \\belowdisplayshortskip with extreme values. Use when the preceding line is short.

```latex
\abovedisplayshortskip=-20pt
\belowdisplayshortskip=100pt
\noindent A short last line...
\[ rac{\hbar^2}{2m}\nabla^2\Psi + V(\mathbf{r})\Psi
= -i\hbar \frac{\partial\Psi}{\partial t} \]
... a short concluding line.
```

--------------------------------

### Call a TeX macro with arguments

Source: https://www.overleaf.com/learn/latex/How%20TeX%20macros%20actually%20work%3A%20Part%204

Provide arguments 'alpha' and 'beta' to the \foo macro. These arguments will replace the \#1 and \#2 placeholders in the macro's replacement text.

```tex
\foo{alpha}{beta}
```

--------------------------------

### Correctly Place \caption Outside tabular Environment

Source: https://www.overleaf.com/learn/how-to/Fixing_and_preventing_compile_timeouts

The \caption command should be placed outside the \begin{tabular} environment to avoid fatal errors when the caption package is loaded. This example demonstrates the incorrect placement.

```latex
\documentclass{article}
\usepackage{caption}
\begin{document}

\begin{table}
    \begin{tabular}{c|c}
        \caption{Caption}% \caption{...} should be OUTSIDE the tabular environment
        a & b \\
        c & d \\
    \end{tabular}
\end{table}
\end{document}
```

--------------------------------

### Coloring Alternating Rows in a Table

Source: https://www.overleaf.com/learn/latex/Tables%23Colouring_a_table_.28cells.2C_rows.2C_columns_and_lines.29

Apply alternating colors to table rows using the \rowcolors command from the xcolor package with the 'table' option. Specify the starting row and colors for odd and even rows.

```latex
\documentclass{article}
\usepackage[table]{xcolor}
\setlength{\arrayrulewidth}{0.5mm}
\setlength{\tabcolsep}{18pt}
\renewcommand{\arraystretch}{2.5}
\begin{document}
{\rowcolors{3}{green!80!yellow!50}{green!70!yellow!40}
\begin{tabular}{ |p{3cm}|p{3cm}|p{3cm}|  }
\hline
\multicolumn{3}{|c|}{Country List} \\
\hline
Country Name or Area Name& ISO ALPHA 2 Code &ISO ALPHA 3 \\
\hline
Afghanistan & AF &AFG \\
Aland Islands & AX   & ALA \\
Albania &AL & ALB \\
Algeria    &DZ & DZA \\
American Samoa & AS & ASM \\
Andorra & AD & AND   \\
Angola & AO & AGO \\
\hline
\end{tabular}}
\end{document}
```

--------------------------------

### Include a LaTeX file using \input

Source: https://www.overleaf.com/learn/latex/Management_in_a_large_project%23Inputting_and_including_files

Use \input{filename} to insert the content of filename.tex. The file should not contain LaTeX preamble code. LaTeX will not start a new page before processing the inputted material.

```latex
\input{filename}
```

--------------------------------

### Importing the multicol Package

Source: https://www.overleaf.com/learn/latex/Multiple_columns

To use the multicol environment, you must include \usepackage{multicol} in your document's preamble.

```latex
\usepackage{multicol}

```

--------------------------------

### Colorizing Hindi, Arabic, and Diacritics with luacolor

Source: https://www.overleaf.com/learn/latex/Using_colours_in_LaTeX

This example demonstrates using the luacolor package to colorize text in Hindi and Arabic, including diacritics. It requires babel and xcolor packages for language support and named colors.

```latex
\documentclass{article}
  
% Prefer a small page width for the demo
\usepackage[paperwidth=15cm]{geometry}
  
% Use babel with Arabic, Hindi and English languages 
% English is loaded last, making it the default language
\usepackage[hindi,english]{babel}
\babelprovide[import=ar]{arabic}
% Set Roman font for Arabic
\babelfont[arabic]{rm}{Scheherazade}
  
% Set Roman and sans serif fonts for English
\babelfont{rm}{Noto Serif}
\babelfont{sf}{Noto Sans}
  
% Set up Roman and san serif fonts for Hindi
\babelfont[hindi]{rm}[Language=Default]{Noto Serif Devanagari}
\babelfont[hindi]{sf}[Language=Default]{Noto Sans Devanagari}

% Because we want to use the "dvipsnames" option to 
% access additional named colors, we must load xcolor
% before luacolor (see the luacolor package documentation)
\usepackage[dvipsnames]{xcolor}
\usepackage{luacolor}
   
% Create a convenience command to typeset Hindi
\newcommand\hinditext[1]{\foreignlanguage{hindi}{#1}}

% Create a convenience command to typeset Arabic
\newcommand\arabictext[1]{\foreignlanguage{arabic}{#1}}

\usepackage{hyperref}
\hypersetup{
colorlinks=true,
urlcolor=cyan
}
\begin{document}
% The Noto fonts benefit from a larger line spacing
\setlength{\baselineskip}{14bp}
  
\section{Colorizing Hindi text}
Google translates the Hindi word \textsf{\hinditext{किंकर्तव्यविमूढ़}} as
``bewildered''. By using the \texttt{luacolor} package it's possible to colorize glyphs within Hindi text; for example \hinditext{किंक\textcolor{purple}{र्तव्यव}\textcolor{green}{िमूढ़}}. You can also add color to the whole word: \hinditext{\textcolor{blue}{किंकर्तव्यविमूढ़}}.

\section{Colorizing Arabic text}
% Create a custom environment to
% typeset colored Arabic text
\newenvironment{colorarabic}
{% Typeset Arabic using a larger font 
\fontsize{30}{30}
% Set right-to-left paragraph and text directions
\pardir TRT\textdir TRT}
{}

Here is some colorized Arabic text:  \begin{colorarabic}
\textcolor{red}{\arabictext{هَذَا}} \arabictext{نَصٌّ عَرَب}\textcolor{green}{\arabictext{ِ}}\arabictext{ي}\textcolor{blue}{\arabictext{ٌّ}}
\end{colorarabic}
\section{Colorizing diacritics}
This example is based on \href{https://tex.stackexchange.com/questions/698933/color-breaks-diacritics-stacking}{code from tex.stackexchange}.

\vspace{12pt}
\bgroup
\newcommand{\emptydiacritic}{\char"034F}
\fontsize{60}{60}\selectfont
á̀̐{\color{blue}a\emptydiacritic\color{green}́\color{red}̀\color{magenta}̐
\egroup
\end{document}
```

--------------------------------

### Define a custom LaTeX package

Source: https://www.overleaf.com/learn/latex/Management_in_a_large_project%23Importing_files

Use this to define custom commands, styles, and environments for your LaTeX document. Place this code in a .sty file and include it using \usepackage{your_package_name}.

```latex
\ProvidesPackage{example}

\usepackage{amsmath}
\usepackage{amsfonts}
\usepackage{amssymb}
\usepackage[latin1]{inputenc}
\usepackage[spanish, english]{babel}
\usepackage{graphicx}
\usepackage{blindtext}
\usepackage{textcomp}
\usepackage{pgfplots}

\pgfplotsset{width=10cm,compat=1.9}

%Header styles
\usepackage{fancyhdr}
\setlength{\headheight}{15pt}
\pagestyle{fancy}
\renewcommand{\chaptermark}[1]{\markboth{#1}{}}
\renewcommand{\sectionmark}[1]{\markright{#1}{}}
\fancyhf{}
\fancyhead[LE,RO]{\thepage}
\fancyhead[RE]{\textbf{\textit{\nouppercase{\leftmark}}}}
\fancyhead[LO]{\textbf{\textit{\nouppercase{\rightmark}}}}
\fancypagestyle{plain}{ % 
\fancyhf{} % remove everything
\renewcommand{\headrulewidth}{0pt} % remove lines as well
\renewcommand{\footrulewidth}{0pt}}

%makes available the commands \proof, \qedsymbol and \theoremstyle
\usepackage{amsthm}

%Ruler
\newcommand{\HRule}{\rule{\linewidth}{0.5mm}}

%Lemma definition and lemma counter
\newtheorem{lemma}{Lemma}[section]

%Definition counter
\theoremstyle{definition}
\newtheorem{definition}{Definition}[section]

%Corolary counter
\newtheorem{corolary}{Corolary}[section]

%Commands for naturals, integers, topology, hull, Ball, Disc, Dimension, boundary and a few more
\newcommand{\E}{{\mathcal{E}}}
\newcommand{\F}{{\mathcal{F}}}
...

%Example environment
\theoremstyle{remark}
\newtheorem{examle}{Example}

%Example counter
\newcommand{\reiniciar}{\setcounter{example}{0}}

```

--------------------------------

### Change Font for Specific Text Element

Source: https://www.overleaf.com/learn/latex/Font_typefaces

Use the `\fontfamily{fontcode}\selectfont` command within braces to change the typeface for a specific section of text. For example, `qcr` selects the TeX Gyre Cursor font.

```latex
\documentclass{article}
\usepackage[T1]{fontenc}
\usepackage{tgbonum}

\begin{document}
This document is a sample document to 
test font families and font typefaces.

{\fontfamily{qcr}\selectfont
This text uses a different font typeface
}
\end{document}


```

--------------------------------

### Main LaTeX File Structure

Source: https://www.overleaf.com/learn/latex/Multi-file_LaTeX_projects

This is the main file for a multi-file LaTeX project. It uses the 'standalone' package for sub-preambles and the 'import' command to include other files.

```latex
\documentclass{article}
\usepackage[subpreambles=true]{standalone}
\usepackage{import}

\title{Standalone package example}
\author{Overleaf}
\date{May 2021}

\begin{document}

\maketitle

\section{First section}
\import{sections/}{introduction}

\section{Second section}
\import{sections/}{section2}

\end{document}
```

--------------------------------

### Correct \vspace command

Source: https://www.overleaf.com/learn/latex/Errors/Missing_number%2C_treated_as_zero

Ensure numerical arguments are provided for spacing commands to function correctly.

```latex
We want to insert some vertical space between here

\vspace{6em}

and here.
```

--------------------------------

### Apply multiple change files with TIE

Source: https://www.overleaf.com/learn/latex/How_Overleaf_created_the_TeX_primitive_reference_data

Merge multiple change files sequentially into a master WEB file to create a new composite WEB file. The order of change files is critical for successful merging.

```bash
tie -m newmytex.web tex.web myprim.ch moreprim.ch
```

--------------------------------

### Basic TeX Macro Definition Structure

Source: https://www.overleaf.com/learn/latex/Articles/What_is_a_TeX_token_list

Illustrates the general syntax for defining a macro in TeX, including the macro name, parameter text, and replacement text.

```tex
\def\<macro name><parameter text>{<replacement text>}

```

--------------------------------

### Load luaotfload Package in Plain TeX

Source: https://www.overleaf.com/learn/latex/Articles/An_overview_of_technologies_supporting_the_use_of_colour_emoji_fonts_in_LaTeX

Include this line in your plain TeX document to enable luaotfload functionality.

```tex
\input luaotfload.sty
```

--------------------------------

### Nested Subscripts in LaTeX

Source: https://www.overleaf.com/learn/latex/Errors/Double_subscript

Illustrates how to create multiple levels of subscripts by nesting braces.

```latex
a_{b_{c_{d_e}}}
```

--------------------------------

### Forgetting to Load a Package

Source: https://www.overleaf.com/learn/latex/Errors/Undefined_control_sequence

Demonstrates an 'Undefined control sequence' error caused by using a command (\xspace) without loading the necessary package (xspace) in the preamble. Include \usepackage{xspace} to fix.

```latex
I want to include a space after the word \LaTeX\xspace but I have forgotten to load the xspace package.

```

--------------------------------

### Linking to Local Files and URLs in LaTeX

Source: https://www.overleaf.com/learn/latex/Hyperlinks

Use `\href` or `\url` to create links to external websites or local files. For local files, prefix the path with `run:`. The path follows UNIX conventions.

```latex
For further references see \href{http://www.overleaf.com}{Something Linky} 
or go to the next url: \url{http://www.overleaf.com} or open the next 
file \href{run:./file.txt}{File.txt}
```

--------------------------------

### Controlling Paragraph Indentation with \parindent and \noindent

Source: https://www.overleaf.com/learn/latex/Paragraphs_and_new_lines

Illustrates how LaTeX typically does not indent the first paragraph in a section. This example demonstrates setting \parindent, indenting subsequent paragraphs, and using \noindent to prevent indentation for a specific paragraph.

```latex
\setlength{\parindent}{20pt}

\section*{This is a section}
\textbf{First paragraph} of a section which, as you can see, is not indented. This is more text in the paragraph. This is more text in the paragraph.

\textbf{Second paragraph}. As you can see it is indented. This is more text in the paragraph. This is more text in the paragraph. 

\noindent\textbf{Third paragraph}. This too is not indented due to use of \texttt{\string\noindent}. This is more text in the paragraph. This is more text in the paragraph.  The current value of \verb|\parindent| is \the\parindent. This is more text in the paragraph.

```

--------------------------------

### Customizing List Labels with Emoji in LuaLaTeX

Source: https://www.overleaf.com/learn/latex/Lists

An example demonstrating how to customize list labels using custom commands and counter variables, specifically designed for LuaLaTeX to render emoji. This requires a color font that supports emoji.

```latex
\renewcommand{\labelenumi}{\duck{enumi}}
\renewcommand{\labelenumii}{\duck{enumi}.\duckegg{enumii}}
\renewcommand{\labelenumiii}{\duck{enumi}.\duckegg{enumii}.\duckegg{enumiii}}
\renewcommand{\labelenumiv}{\duck{enumi}.\duckegg{enumii}.\duckegg{enumiii}.\duckchick{enumiv}} 

\begin{enumerate}
\item A duck
\item More ducks
\item A flurry of ducks
\begin{enumerate}
    \item Ducks and eggs
    \begin{enumerate}
    \item Do I see... 
    \item Ducks and pre-ducks 
       \begin{enumerate}
       \item Awww...
       \item So cute!
       \end{enumerate}
    \end{enumerate}
\end{enumerate}
\item Back to ducks
\item Again
\end{enumerate}


```

--------------------------------

### Define Picture Environment Size

Source: https://www.overleaf.com/learn/latex/Picture_environment

Declare a picture environment with specified width and height in units of \unitlength. Optionally, set the origin offset.

```latex
\begin{picture}(width, height)(Xoffset, Yoffset)
 ...
\end{picture}
```

--------------------------------

### Inserting Floating Elements in Multicolumns

Source: https://www.overleaf.com/learn/latex/Multiple_columns%23Inserting_vertical_rulers

This example shows how to insert floating elements like figures and tables within a multicolumn document using `wrapfig` and `wraptable`. Note that floats in the `multicol` package have limited support, and this is a workaround.

```latex
\begin{multicols}{2}
[
\section{First Section}
All human things are subject to decay. And when fate summons, Monarchs must obey.
]

Hello, here is some text without a meaning.  This text should show what 
a printed text will look like at this place.
If you read this text, you will get no information.  Really?  Is there 
no information?  Is there.

\vfill

\begin{wrapfigure}{l}{0.7\linewidth}
\includegraphics[width=\linewidth]{overleaf-logo}
\caption{This is the Overleaf logo}
\end{wrapfigure}

A blind text like this gives you information about the selected font, how 
the letters are written and an impression of the look.  This text should
contain all...

\begin{wraptable}{l}{0.7\linewidth}
\centering
\begin{tabular}{|c|c|}
\hline
Name & ISO \\
\hline
Afghanistan & AF \\
Aland Islands & AX \\
Albania    &AL  \\
Algeria   &DZ \\
American Samoa & AS \\
Andorra & AD   \\
Angola & AO \\
\hline
\end{tabular}
\caption{Table, floating element}
\label{table:ta}
\end{wraptable}

\end{multicols}
```

--------------------------------

### Feynman Diagram with Invisible Edge for Parallel Photons

Source: https://www.overleaf.com/learn/latex/Feynman_diagrams

Demonstrates using an invisible edge (with `opacity=0.2` for visualization) to ensure two photon edges are parallel. This technique influences the layout algorithm's vertex placement.

```latex
% Invisible edge ensures photons are parallel
\feynmandiagram [small, horizontal=a to t1] {
  a [particle=\(\pi^{0}\)] -- [scalar] t1 -- t2 -- t3 -- t1,
  t2 -- [photon] p1 [particle=\(\gamma\)],
  t3 -- [photon] p2 [particle=\(\gamma\)],
  p1 -- [opacity=0.2] p2,
};

```

--------------------------------

### Using \subimport for Nested File Inclusion

Source: https://www.overleaf.com/learn/latex/Management_in_a_large_project%23Importing_files

This example shows how a file imported into the main document can itself import other files. \subimport is used here to include a plot file, with the path relative to the importing file ('section1-1.tex'), not the main document.

```latex
\section{First section}

Below is a simple 3d plot

\begin{figure}[h]
\centering
\subimport{img/}{plot1.tex}
\caption{Caption}
\label{fig:my_label}
\end{figure}

[...]
```

--------------------------------

### Changing Footnote Numbering Style to Roman Numerals

Source: https://www.overleaf.com/learn/latex/Footnotes

Customizes the appearance of footnote markers by redefining the \thefootnote command. This example changes the numbering style to lowercase Roman numerals using \renewcommand{\thefootnote}{\roman{footnote}}.

```latex
I'm writing to test\footnote{Footnotes work fine!} several footnote features. 
You can insert the footnote marker\footnotemark{} using the \verb|\footnotemark|
command and later use the \verb|\footnotetext| command to typeset the footnote
text by writing \verb|\footnotetext{Text of second footnote.}|
\footnotetext{Text of second footnote.}.

I can use the same footnote\footnotemark{} more than 
once\footnotemark[\value{footnote}].

\footnotetext{A footnote with two references.}

\renewcommand{\thefootnote}{\roman{footnote}}
Now a footnote marker using lowercase Roman numerals\footnote{This footnote marker uses lowercase Roman numerals.}.
```

--------------------------------

### Call Macro with Braced Arguments

Source: https://www.overleaf.com/learn/latex/How%20TeX%20macros%20actually%20work%3A%20Part%204

Illustrates calling a macro defined with parameters and delimiters, using braces to explicitly group arguments. This is an alternative to relying solely on delimiters for argument separation.

```tex
\foo A\bob{This}B\anne{That}\jane{Other}bye!

```

--------------------------------

### LaTeX Error Cascade Example

Source: https://www.overleaf.com/learn/latex/Errors/Missing_%24_inserted

This shows the cascade of errors that can result from the initial 'Missing $ inserted' error, demonstrating TeX's error recovery mechanism when encountering invalid commands like '\par' in math mode.

```latex
! Missing $ inserted.
<inserted text>
                $
l.12 $$y=f(x)\par
                 $$
I’ve inserted a begin-math/end-math symbol since I think
you left one out. Proceed, with fingers crossed.

! Display math should end with $$.
<to be read again>
                   \par 
l.12 $$y=f(x)\par
                 $$
The `$\' that I just saw supposedly matches a previous `$$'.
So I shall assume that you typed `$$' both times.

! Missing $ inserted.
<inserted text>
                $
l.13 \end{document}
                   
I've inserted a begin-math/end-math symbol since I think
you left one out. Proceed, with fingers crossed.

! Display math should end with $$.
<to be read again>
                   \par 
l.13 \end{document}
                   
The `$\' that I just saw supposedly matches a previous `$$'.
So I shall assume that you typed `$$' both times.

```

--------------------------------

### Configure Headers and Footers with \fancyhf

Source: https://www.overleaf.com/learn/latex/Headers_and_footers

Use \fancyhf to set content for specific header and footer locations on even/odd pages. Ensure \pagestyle{fancy} is set before configuration.

```latex
\begin{document}
\pagestyle{fancy}
\fancyhf{}
\fancyhf[EHC]{Even+Header+Centre}
\fancyhf[EFC]{Even+Footer+Centre}
\fancyhf[OFC]{Odd+Footer+Centre}
\fancyhf[OHC]{Odd+Header+Centre}
\lipsum[1]\newpage\lipsum[1]


```

--------------------------------

### 8-bit Engines Active Character Token Calculation Example

Source: https://www.overleaf.com/learn/latex/Articles/An_Introduction_to_LuaTeX_%28Part_2%29%3A_Understanding_%5Cdirectlua

Demonstrates the token calculation for an active character in older 8-bit TeX engines. The active character '~' (character code 126) results in a token value of 4222.

```tex
curcs=character code+1
active character token=curcs+4095
```

--------------------------------

### Basic Table with HTML Colors and Alignment

Source: https://www.overleaf.com/learn/latex/Positioning_of_Figures

Create a basic table with custom rule colors and cell background colors. The 's' column type is used for centered text, and 'p' for paragraph columns.

```latex
Praesent in sapien. Lorem ipsum dolor sit amet, consectetuer adipiscing elit. Duis fringilla tristique neque. 
Sed interdum libero ut metus. Pellentesque placerat. Nam rutrum augue a leo. Morbi sed elit sit amet 
ante lobortis sollicitudin.

\arrayrulecolor[HTML]{DB5800}
\begin{tabular}{ |s|p{2cm}|p{2cm}|  }
\hline
\rowcolor{lightgray} \multicolumn{3}{|c|}{Country List} \\
\hline
Country Name or Area Name& ISO ALPHA 2 Code &ISO ALPHA 3 \\
\hline
Afghanistan & AF &AFG \\
\rowcolor{gray}
Aland Islands & AX  & ALA \\
Albania    &AL & ALB \\
Algeria   &DZ & DZA \\
American Samoa & AS & ASM \\
Andorra & AD & \cellcolor[HTML]{AA0044} AND \\
Angola & AO & AGO \\
\hline
\end{tabular}

Praesent in sapien. Lorem ipsum dolor sit amet, consectetuer adipiscing 
elit. Duis fringilla tristique neque. Sed interdum libero ut metus. Pellentesque placerat. Nam rutrum augue a leo. 
Morbi sed elit sit amet ante lobortis sollicitudin.
```

--------------------------------

### Typesetting Mathematical Expressions in LaTeX

Source: https://www.overleaf.com/learn/latex/Creating_a_document_in_LaTeX

Demonstrates various mathematical notations like subscripts, superscripts, integrals, fractions, and Greek letters. Use this for complex mathematical content.

```latex
\documentclass{article}
\begin{document}
Subscripts in math mode are written as $a_b$ and superscripts are written as $a^b$. These can be combined and nested to write expressions such as

\[ T^{i_1 i_2 \dots i_p}_{j_1 j_2 \dots j_q} = T(x^{i_1},\dots,x^{i_p},e_{j_1},\dots,e_{j_q}) \]
 
We write integrals using $\int$ and fractions using $\frac{a}{b}$. Limits are placed on integrals using superscripts and subscripts:

\[ \int_0^1 \frac{dx}{e^x} =  \frac{e-1}{e} \]

Lower case Greek letters are written as $\omega$ $\delta$ etc. while upper case Greek letters are written as $\Omega$ $\Delta$.

Mathematical operators are prefixed with a backslash as $\sin(\beta)$, $\cos(\alpha)$, $\log(x)$ etc.
\end{document}


```

--------------------------------

### Call Macro with Braced Arguments

Source: https://www.overleaf.com/learn/latex/Understanding_TeX_macros%3A_Part_4?preview=true

Illustrates calling a macro defined with mixed delimiters and parameters, using braces to explicitly group arguments.

```tex
\foo A\bob{This}B\anne{That}\jane{Other}bye!


```

--------------------------------

### Include Images with Alt Text and Caption

Source: https://www.overleaf.com/learn/latex/Articles/How_to_write_in_Markdown_on_Overleaf

Demonstrates the Markdown syntax for including images, specifying alt text, filename, and an optional image caption.

```markdown
![alt-text](file-name "image caption")
```

--------------------------------

### Drawing Ovals, Lines, and Circles with put

Source: https://www.overleaf.com/learn/latex/Picture_environment

Demonstrates drawing basic shapes like lines, circles, and ovals using \line, \circle, and \oval commands, all enclosed within \put commands.

```latex
\documentclass{article}
\usepackage[pdftex]{pict2e}
\begin{document}
\setlength{\unitlength}{1cm}
\thicklines
\begin{picture}(10,6)
\put(2,2.2){\line(1,0){6}}
\put(2,2.2){\circle{2}}
\put(6,2.2){\oval(4,2)[r]}
\end{picture}
\end{document}
```

--------------------------------

### Manually insert a blank page with empty style

Source: https://www.overleaf.com/learn/latex/Headers_and_footers

This method manually inserts a blank page at the end of a chapter to ensure the next chapter starts on a right-hand page. It uses `\clearpage`, `\begingroup`, `\pagestyle{empty}`, and `\cleardoublepage`.

```latex
\clearpage
\begingroup
\pagestyle{empty}\cleardoublepage
\endgroup


```

--------------------------------

### TeX Box Primitives

Source: https://www.overleaf.com/learn/latex/Articles/Pandora%E2%80%99s_%5Chbox%3A_Using_LuaTeX_to_Lift_the_Lid_of_TeX_Boxes

These are the fundamental TeX commands for constructing boxes. \hbox creates horizontal boxes, while \vbox, \vtop, and \vcenter are used for vertical boxes.

```tex
\hbox{...}
```

```tex
\vbox{...}
```

```tex
\vtop{...}
```

```tex
\vcenter{...}
```

--------------------------------

### Custom Bullets with MetaPost and enumitem

Source: https://www.overleaf.com/learn/latex/Lists

Create a custom list environment with unique bullet symbols defined using MetaPost code. This example showcases different custom bullet shapes like dots, yin-yang symbols, and squares.

```latex
\newlist{todolist}{itemize}{2}

\begin{itemize}
  \item Start thinking about what we hope to achieve
  \begin{todolist}
  \item[\mpdot] Identify objectives
  \item[\mpyingyang] Balance environmental impact 
  \item[\mpsquare{0}{5}{0}] Implement plans
    \begin{todolist}
    \item[\mpsquare{-0.5}{4}{0}] Stage 1 plans
    \item[\mpsquare{-0.5}{4}{-20}] Stage 2 plans
    \item[\mpsquare{-0.5}{4}{-40}] Stage 3 plans
    \item[\mpsquare{-0.5}{4}{-60}] Stage 4 plans
    \end{todolist}
  \end{todolist}
\end{itemize}

```

--------------------------------

### Activate EncTeX with pdfTeX

Source: https://www.overleaf.com/learn/latex/MLTeX_SyncTeX_and_EncTeX_TeX_extensions

Create an EncTeX-enabled format file by running pdfTeX in INI mode with the `-enc` option. This enables flexible input/output reencoding.

```bash
pdftex -ini -enc *pdfetex.ini
```

--------------------------------

### Fancyhdr Package: Page X of Y Footer

Source: https://www.overleaf.com/learn/latex/Page_numbering

This example uses the fancyhdr and lastpage packages to display the current page number in the format 'Page X of Y' in the footer. Ensure the geometry package is used for specific page dimensions if needed.

```latex
\documentclass{article}
% Use a small page size to avoid 
% needing lots of text to create 
% several pages
\usepackage[includefoot,
paperheight=10cm,
paperwidth=10cm,
textwidth=9cm,
textheight=8cm]{geometry}
\usepackage{blindtext}
\usepackage{lastpage}
\usepackage{fancyhdr}
\pagestyle{fancy}
\fancyhf{} % clear existing header/footer entries
% Place Page X of Y on the right-hand
% side of the footer
\fancyfoot[R]{Page \thepage \hspace{1pt} of \pageref{LastPage}}
\begin{document}
% Insert a table of contents
\tableofcontents
\newpage
\section{One}
\blindtext[1]
\newpage
\section{Two}
\blindtext[1]
\newpage
\section{Three}
\blindtext[1]
\newpage
\section{Four}
\blindtext[1]
\newpage
\section{Five}
\end{document}
```

--------------------------------

### Link Counters with \counterwithin* (No Print-Format Redefinition)

Source: https://www.overleaf.com/learn/latex/Counters%23Accessing_and_printing_counter_values

Links the 'exampletwo' counter to the 'section' counter using the starred version of \counterwithin. This links the counters but does not redefine the print-format of \theexampletwo.

```latex
\newcounter{exampletwo}
\counterwithin*{exampletwo}{section}
```

--------------------------------

### Add Borders to a LaTeX Table

Source: https://www.overleaf.com/learn/latex/Creating_a_document_in_LaTeX

Add horizontal rules with `\hline` and vertical rules by including `|` in the `tabular` environment's column specification. This example shows vertical rules between all columns and horizontal rules above the first and below the last row.

```latex
\begin{center}
\begin{tabular}{|c|c|c|}
 \hline
 cell1 & cell2 & cell3 \\ 
 cell4 & cell5 & cell6 \\ 
 cell7 & cell8 & cell9 \\ 
 \hline
\end{tabular}
\end{center}
```

--------------------------------

### Creating an Ordered List in LaTeX

Source: https://www.overleaf.com/learn/latex/Creating_a_document_in_LaTeX

Use the enumerate environment to create ordered lists. Each list entry must be preceded by the \item command, which automatically generates the numeric label.

```latex
\documentclass{article}
\begin{document}
\begin{enumerate}
  \item This is the first entry in our list.
  \item The list numbers increase with each entry we add.
\end{enumerate}
\end{document}
```

--------------------------------

### LuaTeX Active Character Token Calculation Example

Source: https://www.overleaf.com/learn/latex/Articles/An_Introduction_to_LuaTeX_%28Part_2%29%3A_Understanding_%5Cdirectlua

Shows the token calculation for an active character in LuaTeX. The active character '~' results in a token value of 536874097, highlighting LuaTeX's use of larger integer values.

```tex
curcs=3186
active character token=3186+229−1=536874097
```

--------------------------------

### Error in array environment due to extra column

Source: https://www.overleaf.com/learn/latex/Errors/Extra_alignment_tab_has_been_changed_to_%5Ccr

This example illustrates the 'Extra alignment tab' error occurring within a \begin{array} environment. The array is defined with 3 columns, but the first row attempts to use 4.

```latex
\[
\begin{array}{lcl}
g(x) & = & (x+2)^2 & = (x+2)(x+2)\ % This row triggers the error
& = & x^2+4x+4\
\end{array}
\]
```

--------------------------------

### Fixing tabular error by creating a new line

Source: https://www.overleaf.com/learn/latex/Errors/Extra_alignment_tab_has_been_changed_to_%5Ccr

This example resolves the 'Extra alignment tab' error in a \begin{tabular} environment by creating a new table row for the excess data, rather than trying to fit it into the existing row.

```latex
\begin{center}
\begin{tabular}{c|c|c}
   1 & 2 & 3 \ 
   4 &   &   \ 
   5 & 6 & 7 \ 
\end{tabular}
\end{center}
```

--------------------------------

### Drawing Vectors in Picture Environment

Source: https://www.overleaf.com/learn/latex/Picture_environment

Demonstrates the use of the \vector command within the \begin{picture} environment to draw arrows. The \put command is used to position elements.

```latex
\documentclass{article}
\usepackage[pdftex]{pict2e}
\begin{document}
\setlength{\unitlength}{1cm}
\begin{picture}(6,6)      % picture box will be 6cm wide by 6cm tall
  \put(0,0){\vector(2,1){4}}  % for every 2 over this vector goes 1 up
    \put(2,1){\makebox(0,0)[l]{\ first leg}}
  \put(4,2){\vector(1,2){2}}
    \put(5,4){\makebox(0,0)[l]{\ second leg}}  
  \put(0,0){\vector(1,1){6}}
    \put(3,3){\makebox(0,0)[r]{sum\ }}
\end{picture}
\end{document}
```

--------------------------------

### Typesetting an Abstract in LaTeX

Source: https://www.overleaf.com/learn/latex/Creating_a_document_in_LaTeX

Demonstrates the use of the `abstract` environment to create a summary or overview at the beginning of a document. Essential for scientific articles.

```latex
\documentclass{article}
\begin{document}
\begin{abstract}
This is a simple paragraph at the beginning of the 
document. A brief introduction about the main subject.
\end{abstract}
\end{document}


```

--------------------------------

### Render Emoji with Skin Tone Modifiers in LuaHBTeX

Source: https://www.overleaf.com/learn/latex/Articles/An_overview_of_technologies_supporting_the_use_of_colour_emoji_fonts_in_LaTeX

This example demonstrates using LuaHBTeX with the fontspec package and NotoColorEmoji.ttf to render isolated and combined emoji characters, including skin tone variations, by specifying Unicode code points.

```latex
\documentclass{article}
\usepackage{fontspec}
\begin{document}
\newfontfamily\emojifont[Renderer=HarfBuzz,SizeFeatures={Size=20}]{NotoColorEmoji.ttf}
Isolated waving hand: {\emojifont\Uchar"1F44B}\par
Isolated modifier: {\emojifont\Uchar"1F3FD}\par 
Combined result: {\emojifont\Uchar"1F44B\Uchar"1F3FD}
\end{document}

```

--------------------------------

### Control File and Rank Visibility

Source: https://www.overleaf.com/learn/latex/Chess_notation

Manage the display of file and rank labels using `hidefiles`, `showfiles`, and `showranks` keys. Define custom file sets with `\def\myfiles`.

```latex
\documentclass{article}
\usepackage{xskak}
\begin{document}
\newchessgame
\def\myfiles{a,b}
\chessboard[hidefiles=\myfiles,
addpieces=Ra2,
showfiles=a,
showranks=2]
\end{document}
```

--------------------------------

### Run LuaTeX in INI mode to create a format file

Source: https://www.overleaf.com/learn/latex/Articles/The_two_modes_of_TeX_engines%3A_INI_mode_and_production_mode

Use this command to instruct LuaTeX to process an .ini file and generate a .fmt file. This is typically done once to create a custom format.

```bash
luatex --ini lualatex.ini
```

--------------------------------

### Controlling Math Style and Spacing

Source: https://www.overleaf.com/learn/latex/Fractions_and_Binomials

Shows how to explicitly control the typesetting style (text, display, script, scriptscript) of fractions and other mathematical elements using commands like \textstyle, \displaystyle, \scriptstyle, and \scriptscriptstyle. This affects size and spacing.

```latex
\documentclass{article}
% Using the geometry package to reduce
% the width of help article graphics
\usepackage[textwidth=9.5cm]{geometry}
\begin{document}

Fractions typeset within a paragraph typically look like this: \(\frac{3x}{2}\). You can force \LaTeX{} to use the larger display style, such as \( \displaystyle \frac{3x}{2} \), which also has an effect on line spacing. The size of maths in a paragraph can also be reduced: \(\scriptstyle \frac{3x}{2}\) or \(\scriptscriptstyle \frac{3x}{2}\). For the \verb|\scriptscriptstyle| example note the reduction in spacing: characters are moved closer to the \textit{vinculum} (the line separating numerator and denominator).

Equally, you can change the style of mathematics normally typeset in display style:

\[f(x)=\frac{P(x)}{Q(x)}\quad \textrm{and}\quad \textstyle f(x)=\frac{P(x)}{Q(x)}\quad \textrm{and}\quad \scriptstyle f(x)=\frac{P(x)}{Q(x)}\]
\end{document}
```

--------------------------------

### Typesetting Arabic with arabtex and pdfLaTeX

Source: https://www.overleaf.com/learn/latex/International_language_support

For right-to-left languages like Arabic with pdfLaTeX, use the `arabtex` package. Ensure `\usepackage[utf8]{inputenc}` is included as `arabtex` depends on it. This example demonstrates typesetting Arabic text within a report structure.

```latex
\documentclass[11pt,a4paper]{report}
\usepackage{arabtex}
\usepackage[utf8]{inputenc}
\usepackage[LFE,LAE]{fontenc}
\usepackage[arabic]{babel}
\title{
\Huge\textsc{اللغة العربية}
}
\author{سالم البوزيدي}
\begin{document}
\maketitle
\tableofcontents
\chapter{علوم الحاسوب}
\section{تاريخ}
\begin{otherlanguage}{arabic}
يعود تاريخ علوم الحاسوب إلى اختراع أول حاسوب رقمي حديث. فقبل العشرينات من القرن العشرين، كان مصطلح حاسوب \textLR{Computer} يشير إلى أي أداة بشرية تقوم بعملية الحسابات. ما هي القضايا أو الأشياء التي يمكن لآلة أن تحسبها باتباع قائمة من التعليمات مع ورقة وقلم، دون تحديد للزمن اللازم ودون أي مهارات أو بصيرة (ذكاء)؟ وكان أحد دوافع هذه الدراسات هو تطوير آلات حاسبة \textLR{computing machines} يمكنها إتمام الأعمال الروتينية والعرضة للخطأ البشري عند إجراء حسابات بشرية.
خلال الأربعينات، مع تطوير آلات حاسبة أكثر قوة وقدرة حسابية، تتطور مصطلح حاسوب ليشير إلى الآلات بدلا من الأشخاص الذين يقومون بالحسابات. وأصبح من الواضح أن الحواسيب يمكنها أن تقوم بأكثر من مجرد عمليات حسابية وبالتالي انتقلوا لدراسة تحسيب أو التحسيب بشكل عام. بدأت المعلوماتية وعلوم الحاسب تأخذ استقلالها كفرع أكاديمي مستقل في الستينات، مع إيجاد أوائل أقسام علوم الحاسب في الجامعات وبدأت الجامعات تعطي إجازات في هذه العلوم [1]. 
\end{otherlanguage}
\begin{thebibliography}{99}
   [1]
    من ويكيبيديا، الموسوعة الحرة
\end{thebibliography}
\end{document}
```

--------------------------------

### Direct Macro Inclusion in \directlua (Error Example)

Source: https://www.overleaf.com/learn/latex/Articles/An_Introduction_to_LuaTeX_%28Part_2%29%3A_Understanding_%5Cdirectlua

Attempting to directly include macro names within a \directlua string without \string or \detokenize will cause TeX to try and expand them, leading to an 'Undefined control sequence' error if they are not defined.

```tex
\directlua{local x = "\foohoo\foo\bar\foobar"}
```

--------------------------------

### LaTeX NAND Gate Implementation

Source: https://www.overleaf.com/learn/latex/Articles/LaTeX_is_More_Powerful_than_you_Think_-_Computing_the_Fibonacci_Numbers_and_Turing_Completeness

Defines a LaTeX command for a NAND gate. This is a fundamental building block for demonstrating Turing completeness.

```latex
\newcount\nanone
\newcount\nantwo

\newcommand{\nand}[2]{
\nanone=#1
\nantwo=#2
  \ifnum\nanone=\nantwo
    \ifnum\nanone=0\relax 1
      \else 0
    \fi
   \else 1
\fi
}
```

--------------------------------

### Missing $ inserted Error with \par in Math Mode

Source: https://www.overleaf.com/learn/latex/Errors/Missing_%24_inserted

This example triggers the 'Missing $ inserted' error because the '\par' command is used within a display math environment ('$$...$$'), where it is not permitted. This highlights that errors can occur even when '$' signs appear balanced.

```latex
\documentclass{article}
\begin{document}
This example generates the error \verb|Missing $ inserted|: 
$$y=f(x)\par$$
\end{document}
```

--------------------------------

### Basic Arabic Word Typesetting with `arabtex`

Source: https://www.overleaf.com/learn/latex/Arabic

Demonstrates typesetting a single Arabic word using the `arabtex` package and UTF-8 encoding. Ensure the `arabtex` and `utf8` packages are included and `\setcode{utf8}` is called.

```latex
\documentclass[a4paper,10pt]{article}
\usepackage{arabtex}
\usepackage{utf8}
\setcode{utf8}
\begin{document}
Here is the word ``Arabic'' written in Arabic:  \<اَلْعَرَبِيَّةُ>. You can also use the command \verb|\RL{arabic text}| like this: \RL{اَلْعَرَبيَّةُ}.
\end{document}

```

--------------------------------

### Error in pmatrix environment with too many columns

Source: https://www.overleaf.com/learn/latex/Errors/Extra_alignment_tab_has_been_changed_to_%5Ccr

This example triggers the 'Extra alignment tab' error in an \amsmath \begin{pmatrix} environment. The error occurs because the number of columns (12) exceeds the default maximum of 10 columns set by \MaxMatrixCols.

```latex
\documentclass{article}
\usepackage{amsmath} % To access the matrix environment
\begin{document}
\[
\begin{pmatrix}
a_{1,1} & a_{1,2} & a_{1,3} & a_{1,4} & a_{1,5} & a_{1,6} & a_{1,7} & a_{1,8} & a_{1,9} & a_{1,10} & a_{1,11} & a_{1,12} \ 
\end{pmatrix}
\]
\end{document}
```

--------------------------------

### TeX expand() Function Implementation

Source: https://www.overleaf.com/learn/latex/Articles/How_does_%5Cexpandafter_work%3A_From_basic_principles_to_exploring_TeX%27s_source_code

Examines the core expand() function in TeX, focusing on the logic for processing \expandafter. It shows how T2 is recursively expanded if expandable, otherwise put back into the input stream.

```c
    **void expand(void)**
    {
    //curcmd is a global variable
    if(curcmd < 111)
    {
      switch(curcmd)
      {
        case **\expandafter**: // Process the \expandafter T1T2 command
        {
            gettoken(); // Read token T1
            t = curtok; // Save token T1 in local variable t
            gettoken(); // Read token T2
            if(curcmd > 100) // Is token T2 expandable?
                **expand();**    // Yes! T2 is expandable:
                             // perform expansion of T2 by
                             // making a **recursive function call** to expand()
            else
                backinput(); // T2 is _not_ expandable: put that token
                             // back in the input to be read again (later)

            curtok = t ;
            backinput() ;
        }
        break;

        // Code to process other expandable commands
        case **“convert to text” command**: // Any one of \number, \string, \romannumeral, 
                                        // \meaning, \fontname, \jobname
                                        // They share the same value of curcmd
        break;

        case **\noexpand**: // Suppress expansion of the next token
        ...
        break;

        case **\csname**:  //Manufacture a control sequence name.
        ...
        break;

        case **\the:** // Insert some tokens
        ....
        break;

        case **“\if... test command”** : // Process one of TeX’s conditionals:  
                                      // \if, \ifcat, \ifnum, \ifdim,\ifodd, \ifvmode, 
                                      // \ifhmode, \ifmmode, \ifinner, \ifvoid, 
                                      // \ifhbox, \ifvbox, \ifx, \ifeof, \iftrue, \iffalse, 
                                      // \ifcase, \ifdefined, \ifcsname, \iffontchar
        ...
        break;

        case **“\fi or \else”**: // Terminate the current conditional
        ...
        break;

        // etc for any other expandable primitive commands supported by
        // the TeX engine

        }

    }
else // Not an expandable primitive: it is a macro
    {
             macrocall()
        }
        //... more code removed
    }

```

--------------------------------

### LaTeX Paragraphs and Line Breaks

Source: https://www.overleaf.com/learn/latex/Creating_a_document_in_LaTeX

Demonstrates creating new paragraphs with blank lines and manual line breaks using \ or \newline. Avoid using multiple line breaks to simulate paragraph spacing.

```latex
\documentclass{article}
\begin{document}

\begin{abstract}
This is a simple paragraph at the beginning of the 
document. A brief introduction about the main subject.
\end{abstract}

After our abstract we can begin the first paragraph, then press ``enter'' twice to start the second one.

This line will start a second paragraph.

I will start the third paragraph and then add \ a manual line break which causes this text to start on a new line but remains part of the same paragraph. Alternatively, I can use the \newline command to start a new line, which is also part of the same paragraph.
\end{document}
```

--------------------------------

### Complex CircuiTikz Diagram with American Voltages

Source: https://www.overleaf.com/learn/latex/CircuiTikz_package

This example showcases a more complex electrical network diagram using standard TikZ syntax and various CircuiTikz nodes like short, V, R, and L, with the 'american voltages' option enabled.

```latex
\documentclass{article}
\usepackage{circuitikz}
\begin{document}
\begin{center}
\begin{circuitikz}[american voltages]
\draw
  (0,0) to [short, *-] (6,0)
  to [V, l_=$\\mathrm{j}\\omega_m \\underline{\\psi}^s_R$] (6,2) 
  to [R, l_=$R_R$] (6,4) 
  to [short, i_=$\\underline{i}^s_R$] (5,4) 
  (0,0) to [open, v^>=$\\underline{u}^s_s$] (0,4) 
  to [short, *- ,i=$\\underline{i}^s_s$] (1,4) 
  to [R, l=$R_s$] (3,4)
  to [L, l=$L_{\\sigma}$] (5,4) 
  to [short, i_=$\\underline{i}^s_M$] (5,3) 
  to [L, l_=$L_M$] (5,0); 
\end{circuitikz}
\end{center}
\end{document}
```

--------------------------------

### Activate MLTeX using -etex option

Source: https://www.overleaf.com/learn/latex/MLTeX_EncTeX_and_SyncTeX_TeX_extensions

An alternative to using the asterisk, this command enables extended mode for e-TeX primitives when creating a format file with MLTeX.

```bash
pdftex -ini -etex -mltex pdfetex.ini

```

--------------------------------

### Extended Arabic Text with Mixed Languages using `arabtex`

Source: https://www.overleaf.com/learn/latex/Arabic

An example demonstrating typesetting a larger section of Arabic text that includes English words using the `\LR{}` command within an `RLtext` environment. Requires `arabtex` and `utf8` packages.

```latex
\documentclass[a4paper,10pt]{article}
\usepackage{arabtex}
\usepackage{utf8}
\begin{document}
\setcode{utf8}
Here is the word ``Arabic'' written in Arabic:  \<اَلْعَرَبِيَّةُ>. You can also use the command \verb|\RL{arabic text}| like this: \RL{اَلْعَرَبيَّةُ}. 

\vspace{10pt}
Here is a larger section of Arabic, containing some words in English within the \verb|\LR| command:

\vspace{10pt}
\begin{RLtext}
يعود تاريخ علوم الحاسوب إلى اختراع أول حاسوب رقمي حديث. فقبل العشرينات من القرن العشرين، كان مصطلح حاسوب \LR{Computer} يشير إلى أي أداة بشرية تقوم بعملية الحسابات. ما هي القضايا أو الأشياء التي يمكن لآلة أن تحسبها باتباع قائمة من التعليمات مع ورقة وقلم، دون تحديد للزمن اللازم ودون أي مهارات أو بصيرة (ذكاء)؟ وكان أحد دوافع هذه الدراسات هو تطوير آلات حاسبة \LR{computing machines} يمكنها إتمام الأعمال الروتينية والعرضة للخطأ البشري عند إجراء حسابات بشرية.
خلال الأربعينات، مع تطوير آلات حاسبة أكثر قوة وقدرة حسابية، تتطور مصطلح حاسوب ليشير إلى الآلات بدلا من الأشخاص الذين يقومون بالحسابات. وأصبح من الواضح أن الحواسيب يمكنها أن تقوم بأكثر من مجرد عمليات حسابية وبالتالي انتقلوا لدراسة تحسيب أو التحسيب بشكل عام. بدأت المعلوماتية وعلوم الحاسب تأخذ استقلالها كفرع أكاديمي مستقل في الستينات، مع إيجاد أوائل أقسام علوم الحاسب في الجامعات وبدأت الجامعات تعطي إجازات في هذه العلوم [1]. 
\end{RLtext}
\end{document}

```

--------------------------------

### View Overleaf's LatexMk File

Source: https://www.overleaf.com/learn/latex/Articles/How_to_use_latexmkrc_with_Overleaf%3A_examples_and_techniques

This LaTeX code snippet, when compiled in Overleaf, copies the system's LatexMk initialization file into your project, allowing you to view and download it.

```latex
\documentclass[a4paper]{article}
\usepackage[margin=1cm]{geometry}
\usepackage{verbatim,shellesc}
\ShellEscape{cp /usr/local/share/latexmk/LatexMk ./LatexMk}
\begin{document}
\section*{About this project}
This project provides access to the system \texttt{LatexMk} initialization (configuration) file used by Overleaf.  \texttt{LatexMk} is a Perl script which may vary slightly according to the \TeX{} Live version and compiler chosen for the project. If you need to be 100\% certain which  \texttt{LatexMk} is being used, add code from this project to your project and compile to typeset a listing of \texttt{LatexMk} and make it available for download as one of the output files.
\section*{Listing the \texttt{LatexMk} file}
\verbatiminput{./LatexMk}
\end{document}
```

--------------------------------

### Table Notes using the `threeparttable` Package

Source: https://www.overleaf.com/learn/latex/Footnotes

The `threeparttable` package offers an alternative to footnotes by creating distinct table notes. It is useful for organizing supplementary information related to a table.

```latex
\begin{table}
  \begin{threeparttable}[b]
   \caption[\TeX{} engine features]{\TeX{} engine feature comparison\tnote{1}}
   \centering
   \begin{tabular}{lcc}
     \midrule 
     \TeX{} engine & Native UTF-8 support & Unicode math support\\
     \midrule
       \hologo{pdfTeX} & No\tnote{2}& No\\
       \Hologo{XeTeX} & Yes & Yes\\
       \Hologo{LuaTeX} & Yes & Yes\\
       \midrule 
     \end{tabular}
     \begin{tablenotes}
       \item [1] This is an early draft.
       \item [2] Some UTF-8 support via \LaTeX{} kernel commands.
     \end{tablenotes}
  \end{threeparttable}
\end{table}


```

--------------------------------

### Using align environment with amsmath package

Source: https://www.overleaf.com/learn/latex/Errors/LaTeX_Error%3A_Environment_XXX_undefined

To use the 'align' environment, you must include '\usepackage{amsmath}' in your preamble. This snippet shows the correct way to load the package and use the environment.

```latex
% In your preamble

\usepackage{amsmath}

% In your main .tex file

\begin{align}
2x + 3y &= 7\
5x - 2y &= 2
\end{align}


```

--------------------------------

### Fix LaTeX Array: Correct Column Alignment

Source: https://www.overleaf.com/learn/latex/Errors%3AExtra_alignment_tab_has_been_changed_to_%5Ccr.

When using the array environment for math, ensure the number of alignment tabs in each row matches the declared number of columns. This example corrects an array that incorrectly uses four columns in its first row.

```latex
\[
\begin{array}{lcl}
g(x) & = & (x+2)^2 \ 
& = & (x+2)(x+2) \ 
& = & x^2+4x+4\ 
\end{array}
\]
```

--------------------------------

### Load graphicx Package in LaTeX

Source: https://www.overleaf.com/learn/latex/Creating_a_document_in_LaTeX

Use the \usepackage command in the preamble to load the graphicx package, which provides commands for importing graphics files.

```latex
\usepackage{graphicx}
```

--------------------------------

### Compile LaTeX to PDF

Source: https://www.overleaf.com/learn/latex/Choosing_a_LaTeX_Compiler%23Other_compilers

Use this command in the system terminal to generate a PDF file from a LaTeX document. Ensure the .tex file is in the current directory.

```bash
pdflatex mydocument.tex
```

--------------------------------

### BibTeX @misc Entry Example for Web Pages

Source: https://www.overleaf.com/learn/latex/Bibliography_management_with_bibtex

Use @misc for entries that don't fit other types, such as web pages. Include 'author', 'title', 'year', and optionally 'note' or 'url'. Ensure 'url' or 'note' packages are loaded in LaTeX preamble.

```bibtex
@misc{web:lang:stats,
  author = {W3Techs},
  title = {Usage Statistics of Content Languages
           for Websites},
  year = {2017},
  note = {Last accessed 16 September 2017},
  url = {http://w3techs.com/technologies/overview/content_language/all}
}


```

--------------------------------

### Configure Beamer with Font Size and TikZ Logo

Source: https://www.overleaf.com/learn/latex/Beamer

Set the document class with a specific font size (e.g., 17pt) and include a TikZ graphic for the logo. This example demonstrates a full Beamer document structure with title page and content frames.

```latex
\documentclass[17pt]{beamer}
\usepackage{tikz}
\usetheme{Madrid}
\usecolortheme{beaver}
\title[About Beamer] %optional
{Madrid theme + beaver}
\subtitle{Demonstrating larger fonts}
\author[Arthur, Doe] % (optional)
{A.~B.~Arthur\inst{1} & J.~Doe\inst{2}}

\institute[VFU] % (optional)
{
  \inst{1}%
  Faculty of Physics\n  Very Famous University
  \and
  \inst{2}%
  Faculty of Chemistry\n  Very Famous University
}

\date[VLC 2021] % (optional)
{Very Large Conference, April 2021}

% Use a simple TikZ graphic to show where the logo is positioned
\logo{\begin{tikzpicture}
\filldraw[color=red!50, fill=red!25, very thick](0,0) circle (0.5);
\node[draw,color=white] at (0,0) {LOGO HERE};
\end{tikzpicture}}
\begin{document}
\frame{\titlepage}
%Highlighting text
\begin{frame}
\frametitle{Demonstrating large fonts}

In this slide, some important text will be
\alert{highlighted} because it's important.
Please, don't abuse it.

\begin{block}{Remark}
Sample text
\end{block}

\end{frame}
\end{document}

```

--------------------------------

### Format One-Line Code Snippet

Source: https://www.overleaf.com/learn/latex/Code_Highlighting_with_minted

Use the \mint command to format single lines of code. The language is specified in braces, and the code itself is delimited by a chosen character (e.g., '|').

```latex
One-line code formatting also works with \texttt{minted}. For example, a small fragment of HTML like this:
\mint{html}|<h2>Something <b>here</b></h2>|
\noindent can be formatted correctly.
```

--------------------------------

### Constructing a Google Drive Direct Download URL for Overleaf

Source: https://www.overleaf.com/learn/how-to/How_can_I_upload_files_from_Google_Drive%3F

Use this template to create a direct download URL for Google Drive files. This URL can be used with Overleaf's 'From External URL' feature. Ensure you correctly extract the FILE_ID and RESOURCE_KEY.

```text
https://drive.google.com/uc?export=download&id=FILE_ID&resourcekey=RESOURCE_KEY
```

--------------------------------

### Typeset Korean with xeCJK and Noto CJK Fonts

Source: https://www.overleaf.com/learn/latex/Korean

Use the xeCJK package with XeLaTeX to typeset Korean text using Google Noto CJK fonts available on Overleaf. This example sets main, CJK main, sans serif, and monospace fonts.

```latex
\documentclass{article}
\usepackage{xeCJK}
\setmainfont{Noto Serif}
\setCJKmainfont{Noto Serif CJK KR}
\setCJKsansfont{Noto Sans CJK KR}
\setCJKmonofont{Noto Sans Mono CJK KR}
\begin{document}
\section{소개}
전체 문서에 대한 기본 정보를 소개 단락.

\begin{verbatim}
그것은 간격 방법을 참조 그대로 글꼴을 테스트
\end{verbatim}

Latin characters are also allowed.
\end{document}

```

--------------------------------

### Electron-Positron Annihilation to Muons

Source: https://www.overleaf.com/learn/latex/Feynman_diagrams

Demonstrates adding momentum arrows and particle labels for electron-positron annihilation into muons. Uses the `momentum` and `particle` style keys.

```latex
\feynmandiagram [horizontal=a to b] {
  i1 [particle=\(e^{-}\] -- [fermion] a -- [fermion] i2 [particle=\(e^{+}\]],
  a -- [photon, edge label=\(\gamma\), momentum'=\(k\)] b,
  f1 [particle=\(\mu^{+}\] -- [fermion] b -- [fermion] f2 [particle=\(\mu^{-}\]],
};

```

--------------------------------

### Lstlisting with Python Syntax Highlighting

Source: https://www.overleaf.com/learn/latex/Code_listing

Enable syntax highlighting for Python code within the `lstlisting` environment by specifying `language=Python`. This formats keywords, comments, and other elements distinctively.

```latex
\begin{lstlisting}[language=Python]
import numpy as np
    
def incmatrix(genl1,genl2):
    m = len(genl1)
    n = len(genl2)
    M = None #to become the incidence matrix
    VT = np.zeros((n*m,1), int)  #dummy variable
    
    #compute the bitwise xor matrix
    M1 = bitxormatrix(genl1)
    M2 = np.triu(bitxormatrix(genl2),1) 

    for i in range(m-1):
        for j in range(i+1, m):
            [r,c] = np.where(M2 == M1[i,j])
            for k in range(len(r)):
                VT[(i)*n + r[k]] = 1;
                VT[(i)*n + c[k]] = 1;
                VT[(j)*n + r[k]] = 1;
                VT[(j)*n + c[k]] = 1;
                
                if M is None:
                    M = np.copy(VT)
                else:
                    M = np.concatenate((M, VT), 1)
                
                VT = np.zeros((n*m,1), int)
    
    return M
\end{lstlisting}
```

--------------------------------

### Set Arabic as the main language with babel

Source: https://www.overleaf.com/learn/latex/Multilingual_typesetting_on_Overleaf_using_babel_and_fontspec

Use \babelprovide with import and main options to set Arabic as the primary language for the document.

```latex
\babelprovide[import,main]{arabic}

```

--------------------------------

### Create a TeX Token List with \toks

Source: https://www.overleaf.com/learn/latex/Articles/What_is_a_TeX_token_list

Use the \toks primitive to create a token list and store it in a token register for later re-use.

```tex
\toks100={Hello}

```

--------------------------------

### Demonstrate \directlua for TeX parameter manipulation in LuaTeX

Source: https://www.overleaf.com/learn/latex/Articles/An_Introduction_to_LuaTeX_%28Part_1%29%3A_What_is_it%E2%80%94and_what_makes_it_so_different%3F

Use \directlua to access and modify TeX parameters like \hsize from within a LaTeX document. This example shows how to read the current \hsize, print it using Lua, and then set a new value for \hsize.

```latex
\documentclass{article}
\begin{document}
\let\
elax %redefine meaning of \ to avoid expansion problems 
Here is the current value of {\ttfamily\string\hsize} (via \LaTeX): 
\the\hsize\par 
\directlua{ 
%Get the current value of \hsize using the Lua API 
local hs=tex.hsize  
% Use a Lua API function to print some  
% LaTeX code and the value of \hsize 
tex.print("Here is the value of {\\ttfamily\\string\\hsize}  
reported from Lua code (in scaled points): ") 
tex.print(hs.."\\par") 
% Set a new value for \hsize using the Lua aPI 
tex.hsize="400pt" % or use tex.hsize=400*65536 (in scaled points) 
}% 
% After \directlua has finished, ask LaTeX  
% to tell us the new value of \hsize 
Here is the value of {\ttfamily\string\hsize} reported  
by \LaTeX{} after {\tt\string\directlua} has finished: 
\the\hsize\par 
\end{document}

```

--------------------------------

### LaTeX: Setting Font Sizes

Source: https://www.overleaf.com/learn/latex/Font_sizes%2C_families%2C_and_styles

Illustrates how to set specific font sizes like \huge and \footnotesize. Sizes are relative to the document's base font size.

```latex
In this example the {\huge huge font size} is set and 
the {\footnotesize Foot note size also}. There's a fairly 
large set of font sizes.

```

--------------------------------

### UTF-8 Input Encoding in LaTeX

Source: https://www.overleaf.com/learn/latex/Arabic

This code snippet specifies UTF-8 as the input encoding for your LaTeX document. It was essential for pdfLaTeX before TeX Live 2018. While often not strictly necessary with newer TeX Live versions, it's still required for the example provided.

```latex
\usepackage[utf8]{inputenc}
```

--------------------------------

### Configure Headers and Footers for All Pages

Source: https://www.overleaf.com/learn/latex/Headers_and_footers

Use \pagestyle{fancy} to enable fancyhdr and \fancyhead{}\fancyfoot{} to clear all headers and footers. Then, specify content for the center header and left footer without indicating odd/even pages, which applies the content to all pages.

```latex
\begin{document}
% Set the page style to "fancy"...
\pagestyle{fancy}
%... then configure it.

% Clear all headers and footers (see also \fancyhf{})
\fancyhead{}
\fancyfoot{}

% Set the Centre header location but do not specify O or E
\fancyhead[C]{In the centre of the header on all pages: \thepage}

% Set the Left footer location but do not specify O or E
\fancyfoot[L]{On the left of the footer on all pages: \thepage}

% Some content:
This is page 1.\newpage
This is page 2.


```

--------------------------------

### Set up Polyglossia and Fontspec for Multilingual Documents

Source: https://www.overleaf.com/learn/latex/Multilingual_typesetting_on_Overleaf_using_polyglossia_and_fontspec

Configure `fontspec` for font selection and `polyglossia` to manage multiple languages. Use `\setdefaultlanguage` and `\setotherlanguages` to declare languages. This setup is essential for documents with text in French, English, Russian, and Thai, using specified typefaces.

```latex
\usepackage{fontspec}
\setmainfont{FreeSerif}
\setsansfont{FreeSans}
\setmonofont{FreeMono}

\usepackage{polyglossia}
\setdefaultlanguage{french}
\setotherlanguages{english,russian,thai}

\begin{document}
\begin{abstract}
Le Lorem Ipsum est simplement du faux texte employé dans 
la composition et la mise en page avant impression.
\end{abstract}

Merci. \textenglish{Thank you.} \textrussian{Спасибо.} Et plus de
texte en français!

Le Lorem Ipsum est le faux texte standard ...

\begin{english}
Lorem Ipsum is simply dummy text ...
\end{english}

\begin{russian}
Lorem Ipsum - это текст-`\textsf{рыба}', часто используемый в 
\texttt{печати} и вэб-дизайне. ...
\end{russian}

\begin{thai}
\XeTeXlinebreaklocale "th_TH"
\textenglish{Lorem Ipsum} คือ เนื้อหาจำลองแบบเรียบๆ ที่ใช้กันในธุรกิจงานพิมพ์หรืองานเรียงพิมพ์
\end{thai}

```
```