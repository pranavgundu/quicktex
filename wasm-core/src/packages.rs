use alloc::vec::Vec;

#[derive(Clone, Copy)]
pub enum Native {
    None,
    Strike { slash: bool, backslash: bool },
    Delimited { matrix: bool },
    Rewrite(fn(&[u8], &mut Vec<u8>), usize),
}

type D = (&'static str, &'static str, &'static str);
type N = (&'static str, &'static str, Native);

pub struct Package {
    pub name: &'static str,
    pub defs: &'static [D],
    pub natives: &'static [N],
}

const QTY: Native = Native::Delimited { matrix: false };
const MQTY: Native = Native::Delimited { matrix: true };

const QTPOW_BODY: &str = r"\IfNoValueF{#1}{^{#1}}";
const ABS_BODY: &str = r"\IfBooleanTF{#1}{\lvert #2\rvert}{\left\lvert #2\right\rvert}";
const BQTY_BODY: &str = r"\left\{#1\right\}";
const COMM_BODY: &str = r"\left[#1,#2\right]";
const ACOMM_BODY: &str = r"\left\{#1,#2\right\}";
const VB_BODY: &str = r"\IfBooleanTF{#1}{\boldsymbol{#2}}{\mathbf{#2}}";
const VA_BODY: &str = r"\IfBooleanTF{#1}{\vec{\boldsymbol{#2}}}{\vec{\mathbf{#2}}}";
const VU_BODY: &str = r"\IfBooleanTF{#1}{\hat{\boldsymbol{#2}}}{\hat{\mathbf{#2}}}";
const GRAD_BODY: &str = r"\nabla\IfNoValueF{#1}{#1}";
const DIV_BODY: &str = r"\nabla\cdot\IfNoValueF{#1}{#1}";
const TR_BODY: &str = r"\operatorname{tr}";
const DD_BODY: &str = r"\mathop{}\!\mathrm{d}\qtpow{#1}\IfNoValueF{#2}{#2}";
const DV_BODY: &str = r"\IfNoValueTF{#3}{\frac{\mathrm{d}\qtpow{#1}}{\mathrm{d}#2\qtpow{#1}}}{\frac{\mathrm{d}\qtpow{#1}#2}{\mathrm{d}#3\qtpow{#1}}}";
const PDV_BODY: &str = r"\IfNoValueTF{#3}{\frac{\partial\qtpow{#1}}{\partial #2\qtpow{#1}}}{\IfNoValueTF{#4}{\frac{\partial\qtpow{#1}#2}{\partial #3\qtpow{#1}}}{\frac{\partial^2 #2}{\partial #3\,\partial #4}}}";
const FDV_BODY: &str = r"\IfNoValueTF{#2}{\frac{\delta}{\delta #1}}{\frac{\delta #1}{\delta #2}}";
const BRA_BODY: &str = r"\left\langle #1\right|";
const KET_BODY: &str = r"\left|#1\right\rangle";
const BRAKET_BODY: &str = r"\IfNoValueTF{#2}{\left\langle #1\middle|#1\right\rangle}{\left\langle #1\middle|#2\right\rangle}";
const DYAD_BODY: &str = r"\IfNoValueTF{#2}{\ket{#1}\bra{#1}}{\ket{#1}\bra{#2}}";
const EV_BODY: &str = r"\IfNoValueTF{#2}{\left\langle #1\right\rangle}{\left\langle #2\middle|#1\middle|#2\right\rangle}";
const MEL_BODY: &str = r"\left\langle #1\middle|#2\middle|#3\right\rangle";
const SI_BODY: &str = r"\num{#2}\,\si{#3}";
const NICEFRAC_BODY: &str = r"{}^{#2}\!/\!{}_{#3}";
const OD_BODY: &str = r"\frac{\mathrm{d}\qtpow{#1}#2}{\mathrm{d}#3\qtpow{#1}}";
const PD_BODY: &str = r"\frac{\partial\qtpow{#1}#2}{\partial #3\qtpow{#1}}";

pub const AMSMATH: usize = 0;

const fn pkg(name: &'static str, defs: &'static [D]) -> Package {
    Package { name, defs, natives: &[] }
}

pub static NOOPS: &[&str] = &[
    "amssymb",
    "amsfonts",
    "amsbsy",
    "amstext",
    "amsopn",
    "amsthm",
    "amscd",
    "latexsym",
    "mathrsfs",
    "eucal",
    "eufrak",
    "bm",
    "color",
    "xcolor",
    "graphicx",
    "inputenc",
    "fontenc",
    "geometry",
    "hyperref",
    "babel",
    "lmodern",
    "microtype",
    "textcomp",
    "array",
    "calc",
    "ifthen",
    "etoolbox",
    "xparse",
    "fixltx2e",
    "float",
    "booktabs",
    "enumitem",
    "setspace",
    "parskip",
    "caption",
    "subcaption",
    "fancyhdr",
    "titlesec",
    "cases",
    "mathptmx",
    "newtxmath",
];

pub static PACKAGES: &[Package] = &[
    pkg(
        "amsmath",
        &[
            ("tag", "m", r"\qquad(\text{#1})"),
            ("dddot", "m", r"\overset{...}{#1}"),
            ("ddddot", "m", r"\overset{....}{#1}"),
            ("iiiint", "", r"\int\!\!\!\int\!\!\!\int\!\!\!\int"),
            ("idotsint", "", r"\int\!\cdots\!\int"),
            ("pmb", "m", r"\boldsymbol{#1}"),
            ("varinjlim", "", r"\mathop{\underrightarrow{\mathrm{lim}}}"),
            ("injlim", "", r"\operatorname*{inj\,lim}"),
            ("projlim", "", r"\operatorname*{proj\,lim}"),
        ],
    ),
    pkg(
        "mathtools",
        &[
            ("vcentcolon", "", r"\mathrel{:}"),
            ("dblcolon", "", r"\mathrel{::}"),
            ("Coloneqq", "", r"\mathrel{::=}"),
            ("xmapsto", "om", r"\overset{#2}{\longmapsto}"),
            ("xleftrightarrow", "om", r"\overset{#2}{\longleftrightarrow}"),
            ("xRightarrow", "om", r"\overset{#2}{\Longrightarrow}"),
            ("xLeftarrow", "om", r"\overset{#2}{\Longleftarrow}"),
            ("xLeftrightarrow", "om", r"\overset{#2}{\Longleftrightarrow}"),
            ("xhookrightarrow", "om", r"\overset{#2}{\hookrightarrow}"),
            ("xhookleftarrow", "om", r"\overset{#2}{\hookleftarrow}"),
            ("prescript", "mmm", r"{}^{#1}_{#2}#3"),
            ("splitfrac", "mm", r"\begin{array}{l}#1\\\quad #2\end{array}"),
            ("adjustlimits", "", ""),
            ("MoveEqLeft", "o", ""),
            ("smashoperator", "om", r"#2"),
        ],
    ),
    Package {
        name: "physics",
        defs: &[
            ("qtpow", "m", QTPOW_BODY),
            ("abs", "sm", ABS_BODY),
            ("absolutevalue", "sm", ABS_BODY),
            ("norm", "sm", r"\IfBooleanTF{#1}{\lVert #2\rVert}{\left\lVert #2\right\rVert}"),
            ("eval", "m", r"\left.#1\right|"),
            ("evaluated", "m", r"\left.#1\right|"),
            ("order", "m", r"\mathcal{O}\left(#1\right)"),
            ("pqty", "m", r"\left(#1\right)"),
            ("bqty", "m", r"\left[#1\right]"),
            ("Bqty", "m", BQTY_BODY),
            ("vqty", "m", r"\left|#1\right|"),
            ("comm", "mm", COMM_BODY),
            ("commutator", "mm", COMM_BODY),
            ("acomm", "mm", ACOMM_BODY),
            ("anticommutator", "mm", ACOMM_BODY),
            ("pb", "mm", ACOMM_BODY),
            ("poissonbracket", "mm", ACOMM_BODY),
            ("vb", "sm", VB_BODY),
            ("vectorbold", "sm", VB_BODY),
            ("va", "sm", VA_BODY),
            ("vectorarrow", "sm", VA_BODY),
            ("vu", "sm", VU_BODY),
            ("vectorunit", "sm", VU_BODY),
            ("vdot", "", r"\cdot"),
            ("dotproduct", "", r"\cdot"),
            ("cross", "", r"\times"),
            ("cp", "", r"\times"),
            ("crossproduct", "", r"\times"),
            ("grad", "g", GRAD_BODY),
            ("gradient", "g", GRAD_BODY),
            ("div", "g", DIV_BODY),
            ("divergence", "g", DIV_BODY),
            ("curl", "g", r"\nabla\times\IfNoValueF{#1}{#1}"),
            ("laplacian", "g", r"\nabla^2\IfNoValueF{#1}{#1}"),
            ("tr", "", TR_BODY),
            ("trace", "", TR_BODY),
            ("Tr", "", r"\operatorname{Tr}"),
            ("rank", "", r"\operatorname{rank}"),
            ("erf", "", r"\operatorname{erf}"),
            ("Res", "", r"\operatorname{Res}"),
            ("PV", "", r"\operatorname{P.V.}"),
            ("Re", "", r"\operatorname{Re}"),
            ("Im", "", r"\operatorname{Im}"),
            ("dd", "og", DD_BODY),
            ("differential", "og", DD_BODY),
            ("dv", "omg", DV_BODY),
            ("derivative", "omg", DV_BODY),
            ("pdv", "omgg", PDV_BODY),
            ("partialderivative", "omgg", PDV_BODY),
            ("fdv", "mg", FDV_BODY),
            ("functionalderivative", "mg", FDV_BODY),
            ("bra", "m", BRA_BODY),
            ("ket", "m", KET_BODY),
            ("braket", "mg", BRAKET_BODY),
            ("ip", "mg", BRAKET_BODY),
            ("innerproduct", "mg", BRAKET_BODY),
            ("dyad", "mg", DYAD_BODY),
            ("ketbra", "mg", DYAD_BODY),
            ("op", "mg", DYAD_BODY),
            ("ev", "mg", EV_BODY),
            ("expval", "mg", EV_BODY),
            ("expectationvalue", "mg", EV_BODY),
            ("mel", "mmm", MEL_BODY),
            ("matrixel", "mmm", MEL_BODY),
            ("matrixelement", "mmm", MEL_BODY),
            ("pmqty", "m", r"\begin{pmatrix}#1\end{pmatrix}"),
            ("bmqty", "m", r"\begin{bmatrix}#1\end{bmatrix}"),
            ("vmqty", "m", r"\begin{vmatrix}#1\end{vmatrix}"),
        ],
        natives: &[("qty", "", QTY), ("quantity", "", QTY), ("mqty", "", MQTY), ("matrixquantity", "", MQTY)],
    },
    pkg(
        "braket",
        &[
            ("bra", "m", r"\langle #1\rvert"),
            ("ket", "m", r"\lvert #1\rangle"),
            ("braket", "m", r"\langle #1\rangle"),
            ("Bra", "m", BRA_BODY),
            ("Ket", "m", KET_BODY),
            ("Braket", "m", r"\left\langle #1\right\rangle"),
            ("set", "m", r"\{#1\}"),
            ("Set", "m", BQTY_BODY),
        ],
    ),
    Package {
        name: "cancel",
        defs: &[("cancelto", "mm", r"\cancel{#2}^{#1}")],
        natives: &[
            ("cancel", "", Native::Strike { slash: true, backslash: false }),
            ("bcancel", "", Native::Strike { slash: false, backslash: true }),
            ("xcancel", "", Native::Strike { slash: true, backslash: true }),
        ],
    },
    Package { name: "mhchem", defs: &[], natives: &[("ce", "m", Native::Rewrite(chemistry, 0))] },
    Package { name: "chemformula", defs: &[], natives: &[("ch", "m", Native::Rewrite(chemistry, 0))] },
    Package {
        name: "siunitx",
        defs: &[
            ("SI", "omm", SI_BODY),
            ("qty", "omm", SI_BODY),
            ("unit", "om", r"\si{#2}"),
            ("ang", "om", r"#2^{\circ}"),
            ("SIrange", "ommm", r"\num{#2}\,\si{#4}\text{ to }\num{#3}\,\si{#4}"),
            ("numrange", "omm", r"\num{#2}\text{ to }\num{#3}"),
        ],
        natives: &[("si", "om", Native::Rewrite(units, 1)), ("num", "om", Native::Rewrite(number, 1))],
    },
    pkg("nicefrac", &[("nicefrac", "omm", NICEFRAC_BODY)]),
    pkg("xfrac", &[("sfrac", "omm", NICEFRAC_BODY)]),
    pkg(
        "gensymb",
        &[("degree", "", r"{}^{\circ}"), ("celsius", "", r"{}^{\circ}\mathrm{C}"), ("ohm", "", r"\Omega"), ("micro", "", r"\mu")],
    ),
    pkg(
        "commath",
        &[
            ("qtpow", "m", QTPOW_BODY),
            ("dif", "", r"\mathop{}\!\mathrm{d}"),
            ("Dif", "", r"\mathop{}\!\mathrm{D}"),
            ("od", "omm", OD_BODY),
            ("pd", "omm", PD_BODY),
            ("abs", "m", r"\left\lvert #1\right\rvert"),
            ("norm", "m", r"\left\lVert #1\right\rVert"),
            ("set", "m", BQTY_BODY),
            ("eval", "m", r"\left.#1\right|"),
        ],
    ),
    pkg(
        "derivative",
        &[
            ("qtpow", "m", QTPOW_BODY),
            ("odv", "omm", OD_BODY),
            ("pdv", "omm", PD_BODY),
            ("fdv", "omm", r"\frac{\delta\qtpow{#1}#2}{\delta #3\qtpow{#1}}"),
            ("mdv", "omm", r"\frac{\mathrm{D}\qtpow{#1}#2}{\mathrm{D}#3\qtpow{#1}}"),
        ],
    ),
    pkg("diffcoeff", &[("qtpow", "m", QTPOW_BODY), ("diff", "omm", OD_BODY), ("diffp", "omm", PD_BODY)]),
    pkg(
        "units",
        &[
            ("unit", "om", r"\IfNoValueF{#1}{#1\,}\mathrm{#2}"),
            ("unitfrac", "omm", r"\IfNoValueF{#1}{#1\,}{}^{\mathrm{#2}}\!/\!{}_{\mathrm{#3}}"),
        ],
    ),
    pkg("dsfont", &[("mathds", "m", r"\mathbb{#1}")]),
    pkg("bbm", &[("mathbbm", "m", r"\mathbb{#1}")]),
    pkg("bbold", &[("mathbbold", "m", r"\mathbb{#1}")]),
    pkg("centernot", &[("centernot", "m", r"\not#1")]),
    pkg("slashed", &[("slashed", "m", r"\mathrlap{/}#1")]),
    pkg("faktor", &[("faktor", "mm", r"{}^{#1}\!\big/\!{}_{#2}")]),
    pkg("interval", &[("interval", "omm", r"\left[#2,#3\right]")]),
    pkg("stmaryrd", &[("llbracket", "", r"\mathopen{[\mkern-3mu[}"), ("rrbracket", "", r"\mathclose{]\mkern-3mu]}")]),
];

pub fn closing_brace(s: &[u8], from: usize) -> usize {
    let mut depth = 0;
    let mut i = from;
    while i < s.len() && (s[i] != b'}' || depth > 0) {
        match s[i] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    i
}

pub fn find_package(name: &[u8]) -> Option<usize> {
    PACKAGES.iter().position(|p| p.name.as_bytes() == name)
}

pub fn is_noop(name: &[u8]) -> bool {
    NOOPS.iter().any(|p| p.as_bytes() == name)
}

fn prefix(name: &[u8]) -> Option<&'static str> {
    Some(match name {
        b"yocto" => "y",
        b"zepto" => "z",
        b"atto" => "a",
        b"femto" => "f",
        b"pico" => "p",
        b"nano" => "n",
        b"micro" => r"\mu ",
        b"milli" => "m",
        b"centi" => "c",
        b"deci" => "d",
        b"deca" | b"deka" => "da",
        b"hecto" => "h",
        b"kilo" => "k",
        b"mega" => "M",
        b"giga" => "G",
        b"tera" => "T",
        b"peta" => "P",
        b"exa" => "E",
        b"zetta" => "Z",
        b"yotta" => "Y",
        _ => return None,
    })
}

fn unit(name: &[u8]) -> Option<&'static str> {
    Some(match name {
        b"meter" | b"metre" => "m",
        b"second" => "s",
        b"gram" => "g",
        b"kilogram" => "kg",
        b"ampere" => "A",
        b"kelvin" => "K",
        b"mole" => "mol",
        b"candela" => "cd",
        b"hertz" => "Hz",
        b"newton" => "N",
        b"pascal" => "Pa",
        b"joule" => "J",
        b"watt" => "W",
        b"coulomb" => "C",
        b"volt" => "V",
        b"ohm" => r"\Omega ",
        b"farad" => "F",
        b"henry" => "H",
        b"tesla" => "T",
        b"weber" => "Wb",
        b"siemens" => "S",
        b"lumen" => "lm",
        b"lux" => "lx",
        b"becquerel" => "Bq",
        b"gray" => "Gy",
        b"sievert" => "Sv",
        b"katal" => "kat",
        b"liter" | b"litre" => "L",
        b"electronvolt" => "eV",
        b"minute" => "min",
        b"hour" => "h",
        b"day" => "d",
        b"radian" => "rad",
        b"steradian" => "sr",
        b"bar" => "bar",
        b"dalton" => "Da",
        b"tonne" => "t",
        b"hectare" => "ha",
        b"neper" => "Np",
        b"bel" => "B",
        b"decibel" => "dB",
        b"percent" => r"\%",
        b"degreeCelsius" => r"{}^{\circ}\mathrm{C}",
        b"degree" => r"{}^{\circ}",
        b"arcminute" => r"{}'",
        b"arcsecond" => r"{}''",
        _ => return None,
    })
}

pub fn number(s: &[u8], out: &mut Vec<u8>) {
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if (c == b'e' || c == b'E') && i > 0 {
            let mut j = i + 1;
            if j < s.len() && (s[j] == b'+' || s[j] == b'-') {
                j += 1;
            }
            let e = s[j..].iter().position(|b| !b.is_ascii_digit()).map_or(s.len(), |n| j + n);
            if e > j {
                out.extend_from_slice(b"\\times10^{");
                let start = if s[i + 1] == b'+' { i + 2 } else { i + 1 };
                out.extend_from_slice(&s[start..e]);
                out.push(b'}');
                i = e;
                continue;
            }
        }
        if c == b'+' && s.get(i + 1) == Some(&b'-') {
            out.extend_from_slice(b"\\pm ");
            i += 2;
            continue;
        }
        if c == b'x' {
            out.extend_from_slice(b"\\times ");
            i += 1;
            continue;
        }
        if c != b' ' {
            out.push(c);
        }
        i += 1;
    }
}

fn write_int(mut v: i32, out: &mut Vec<u8>) {
    if v < 0 {
        out.push(b'-');
        v = -v;
    }
    if v >= 10 {
        write_int(v / 10, out);
    }
    out.push(b'0' + (v % 10) as u8);
}

pub fn units(s: &[u8], out: &mut Vec<u8>) {
    let mut i = 0;
    let mut sign = 1;
    let mut power = 0;
    let mut pre: Option<&str> = None;
    let mut first = true;
    let mut open = false;
    let close = |out: &mut Vec<u8>, open: &mut bool, power: &mut i32, sign: &mut i32| {
        if *open {
            let p = if *power == 0 { 1 } else { *power } * *sign;
            if p != 1 {
                out.extend_from_slice(b"^{");
                write_int(p, out);
                out.push(b'}');
            }
            *open = false;
            *power = 0;
            *sign = 1;
        }
    };
    while i < s.len() {
        if s[i] != b'\\' {
            let e = s[i..].iter().position(|b| *b == b'\\').map_or(s.len(), |n| i + n);
            let part = s[i..e].trim_ascii();
            if !part.is_empty() {
                close(out, &mut open, &mut power, &mut sign);
                if !first {
                    out.extend_from_slice(b"\\,");
                }
                first = false;
                out.extend_from_slice(b"\\mathrm{");
                out.extend_from_slice(part);
                out.push(b'}');
            }
            i = e;
            continue;
        }
        i += 1;
        let st = i;
        while i < s.len() && s[i].is_ascii_alphabetic() {
            i += 1;
        }
        let name = &s[st..i];
        match name {
            b"per" => {
                close(out, &mut open, &mut power, &mut sign);
                sign = -1;
            }
            b"square" => {
                close(out, &mut open, &mut power, &mut sign);
                power = 2;
            }
            b"cubic" => {
                close(out, &mut open, &mut power, &mut sign);
                power = 3;
            }
            b"squared" => power = 2,
            b"cubed" => power = 3,
            b"tothe" => {
                let st = s[i..].iter().position(|b| b.is_ascii_digit()).map_or(s.len(), |n| i + n);
                let e = s[st..].iter().position(|b| !b.is_ascii_digit()).map_or(s.len(), |n| st + n);
                power = s[st..e].iter().fold(0i32, |a, b| a.saturating_mul(10).saturating_add((b - b'0') as i32)).min(9999);
                i = s[e..].iter().position(|b| *b == b'}').map_or(s.len(), |n| e + n + 1);
            }
            _ => {
                if let Some(p) = prefix(name) {
                    pre = Some(p);
                } else if let Some(u) = unit(name) {
                    let (keep_sign, keep_power) = (sign, power);
                    close(out, &mut open, &mut power, &mut sign);
                    sign = keep_sign;
                    power = keep_power;
                    if !first {
                        out.extend_from_slice(b"\\,");
                    }
                    first = false;
                    out.extend_from_slice(b"\\mathrm{");
                    if let Some(p) = pre.take() {
                        out.extend_from_slice(p.as_bytes());
                    }
                    out.extend_from_slice(u.as_bytes());
                    out.push(b'}');
                    open = true;
                } else {
                    out.push(b'\\');
                    out.extend_from_slice(name);
                }
            }
        }
    }
    close(out, &mut open, &mut power, &mut sign);
}

fn script(s: &[u8], mut i: usize, mark: u8, out: &mut Vec<u8>) -> usize {
    out.push(mark);
    out.push(b'{');
    if s.get(i) == Some(&b'{') {
        let st = i + 1;
        let e = closing_brace(s, st);
        out.extend_from_slice(&s[st..e]);
        i = (e + 1).min(s.len());
    } else {
        let st = i;
        while i < s.len() && s[i] != b' ' {
            i += 1;
        }
        out.extend_from_slice(&s[st..i]);
    }
    out.push(b'}');
    i
}

pub fn chemistry(s: &[u8], out: &mut Vec<u8>) {
    let arrows: [(&[u8], &[u8]); 4] =
        [(b"<=>", b"\\rightleftharpoons "), (b"<->", b"\\leftrightarrow "), (b"->", b"\\rightarrow "), (b"<-", b"\\leftarrow ")];
    let mut i = 0;
    let mut bond = false;
    'outer: while i < s.len() {
        for (a, b) in &arrows {
            if s[i..].starts_with(a) {
                out.extend_from_slice(b"\\;");
                out.extend_from_slice(b);
                out.extend_from_slice(b"\\;");
                i += a.len();
                bond = false;
                continue 'outer;
            }
        }
        let c = s[i];
        match c {
            b' ' => {
                i += 1;
                bond = false;
            }
            b'^' | b'_' => i = script(s, i + 1, c, out),
            b'0'..=b'9' => {
                let st = i;
                while i < s.len() && (s[i].is_ascii_digit() || s[i] == b'.') {
                    i += 1;
                }
                if bond {
                    out.extend_from_slice(b"_{");
                    out.extend_from_slice(&s[st..i]);
                    out.push(b'}');
                } else {
                    out.extend_from_slice(&s[st..i]);
                }
            }
            b'A'..=b'Z' | b'a'..=b'z' => {
                let st = i;
                i += 1;
                while i < s.len() && s[i].is_ascii_lowercase() {
                    i += 1;
                }
                out.extend_from_slice(b"\\mathrm{");
                out.extend_from_slice(&s[st..i]);
                out.push(b'}');
                bond = true;
            }
            b')' | b']' => {
                out.push(c);
                i += 1;
                bond = true;
            }
            b'+' | b'-' => {
                let end = i + 1 == s.len() || s[i + 1] == b' ';
                if bond && end {
                    out.extend_from_slice(if c == b'+' { b"^{+}" } else { b"^{-}" });
                } else {
                    out.push(c);
                }
                i += 1;
                bond = false;
            }
            b'*' | b'.' => {
                out.extend_from_slice(b"\\cdot ");
                i += 1;
                bond = false;
            }
            b'\\' => {
                out.push(c);
                i += 1;
                while i < s.len() && s[i].is_ascii_alphabetic() {
                    out.push(s[i]);
                    i += 1;
                }
                out.push(b' ');
                bond = true;
            }
            _ => {
                out.push(c);
                i += 1;
                bond = false;
            }
        }
    }
}
