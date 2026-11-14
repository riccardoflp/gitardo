//! The languages of the colors of the code, told by the names of their files, and what the lexer of code needs of each:
//! its comments, strings, keywords, types, constants and the marks before some names.

use super::TokenKind;
use super::grammar::{Grammar, NAME, Quote};

mod tables;

/// A language the colors of the code know.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    /// TypeScript, and TSX.
    TypeScript,
    /// JavaScript, and JSX.
    JavaScript,
    /// JSON, and JSON with comments.
    Json,
    /// CSS.
    Css,
    /// SCSS.
    Scss,
    /// Less.
    Less,
    /// HTML, Vue, Svelte and XML.
    Markup,
    /// Python.
    Python,
    /// Ruby.
    Ruby,
    /// PHP.
    Php,
    /// Lua.
    Lua,
    /// Java.
    Java,
    /// Kotlin.
    Kotlin,
    /// Swift.
    Swift,
    /// Dart.
    Dart,
    /// C#.
    CSharp,
    /// Go.
    Go,
    /// Rust.
    Rust,
    /// C.
    C,
    /// C++.
    Cpp,
    /// YAML.
    Yaml,
    /// TOML.
    Toml,
    /// INI.
    Ini,
    /// Markdown.
    Markdown,
    /// Shell.
    Shell,
    /// PowerShell.
    PowerShell,
    /// SQL.
    Sql,
    /// Dockerfile.
    Dockerfile,
}

/// Which lexer cuts a language.
pub(crate) enum Lexer {
    /// The lexer of code, with the language's grammar.
    Code(&'static Grammar),
    /// The lexer of HTML and XML.
    Markup,
    /// The lexer of Markdown.
    Markdown,
}

impl Language {
    /// The language of a file, by its name: its extension, or `Dockerfile`; none for a file of no language of the
    /// card.
    #[must_use]
    pub fn of(path: &str) -> Option<Language> {
        let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
        if name == "Dockerfile" {
            return Some(Language::Dockerfile);
        }
        let (_, extension) = name.rsplit_once('.')?;
        let language = match extension.to_ascii_lowercase().as_str() {
            "ts" | "mts" | "cts" | "tsx" => Language::TypeScript,
            "js" | "mjs" | "cjs" | "jsx" => Language::JavaScript,
            "json" | "jsonc" => Language::Json,
            "css" => Language::Css,
            "scss" => Language::Scss,
            "less" => Language::Less,
            "html" | "htm" | "vue" | "svelte" | "xml" | "svg" => Language::Markup,
            "py" => Language::Python,
            "rb" => Language::Ruby,
            "php" => Language::Php,
            "lua" => Language::Lua,
            "java" => Language::Java,
            "kt" | "kts" => Language::Kotlin,
            "swift" => Language::Swift,
            "dart" => Language::Dart,
            "cs" => Language::CSharp,
            "go" => Language::Go,
            "rs" => Language::Rust,
            "c" | "h" => Language::C,
            "cpp" | "cc" | "cxx" | "hpp" | "hh" => Language::Cpp,
            "yml" | "yaml" => Language::Yaml,
            "toml" => Language::Toml,
            "ini" => Language::Ini,
            "md" | "markdown" => Language::Markdown,
            "sh" | "bash" | "zsh" => Language::Shell,
            "ps1" | "psm1" => Language::PowerShell,
            "sql" => Language::Sql,
            "dockerfile" => Language::Dockerfile,
            _ => return None,
        };
        Some(language)
    }

    /// The lexer that cuts it.
    pub(crate) fn grammar(self) -> Lexer {
        let grammar = match self {
            Language::Markup => return Lexer::Markup,
            Language::Markdown => return Lexer::Markdown,
            Language::TypeScript => &TYPESCRIPT,
            Language::JavaScript => &JAVASCRIPT,
            Language::Json => &JSON,
            Language::Css => &CSS,
            Language::Scss => &SCSS,
            Language::Less => &LESS,
            Language::Python => &PYTHON,
            Language::Ruby => &RUBY,
            Language::Php => &PHP,
            Language::Lua => &LUA,
            Language::Java => &JAVA,
            Language::Kotlin => &KOTLIN,
            Language::Swift => &SWIFT,
            Language::Dart => &DART,
            Language::CSharp => &CSHARP,
            Language::Go => &GO,
            Language::Rust => &RUST,
            Language::C => &C,
            Language::Cpp => &CPP,
            Language::Yaml => &YAML,
            Language::Toml => &TOML,
            Language::Ini => &INI,
            Language::Shell => &SHELL,
            Language::PowerShell => &POWERSHELL,
            Language::Sql => &SQL,
            Language::Dockerfile => &DOCKERFILE,
        };
        Lexer::Code(grammar)
    }
}

/// A string between double quotes, on one line, with backslash escapes.
const DOUBLE: Quote = Quote::line("\"", "\"");
/// A string between single quotes, on one line, with backslash escapes.
const SINGLE: Quote = Quote::line("'", "'");
/// The comments of C: `//` and `/* */`.
const C_LINE: &[&str] = &["//"];
/// The block comment of C.
const C_BLOCK: &[(&str, &str)] = &[("/*", "*/")];
/// A decorator or an annotation: `@Override`.
const DECORATOR: &[(u8, TokenKind)] = &[(b'@', TokenKind::Attribute)];

/// The grammar of the languages of C's family, to be given their keywords and the rest.
const C_FAMILY: Grammar = Grammar {
    line_comments: C_LINE,
    block_comments: C_BLOCK,
    quotes: &[DOUBLE, SINGLE],
    ..NAME
};

/// TypeScript and TSX.
const TYPESCRIPT: Grammar = Grammar {
    quotes: &[DOUBLE, SINGLE, Quote::lines("`", "`")],
    keywords: tables::TYPESCRIPT_KEYWORDS,
    types: tables::TYPESCRIPT_TYPES,
    constants: tables::JAVASCRIPT_CONSTANTS,
    marks: DECORATOR,
    ..C_FAMILY
};
/// JavaScript and JSX.
const JAVASCRIPT: Grammar = Grammar {
    types: "",
    ..TYPESCRIPT
};
/// JSON, with the comments of JSON with comments.
const JSON: Grammar = Grammar {
    quotes: &[DOUBLE],
    constants: "true false null",
    keys_before_colons: true,
    capital_types: false,
    bare: TokenKind::Plain,
    ..C_FAMILY
};
/// CSS.
const CSS: Grammar = Grammar {
    line_comments: &[],
    keywords: "important",
    marks: &[
        (b'@', TokenKind::Keyword),
        (b'#', TokenKind::Constant),
        (b'.', TokenKind::Type),
    ],
    keys_before_colons: true,
    has_dashes: true,
    capital_types: false,
    bare: TokenKind::Tag,
    ..C_FAMILY
};
/// SCSS.
const SCSS: Grammar = Grammar {
    line_comments: C_LINE,
    marks: &[
        (b'$', TokenKind::Variable),
        (b'@', TokenKind::Keyword),
        (b'#', TokenKind::Constant),
        (b'.', TokenKind::Type),
    ],
    ..CSS
};
/// Less.
const LESS: Grammar = Grammar {
    line_comments: C_LINE,
    marks: &[
        (b'@', TokenKind::Variable),
        (b'#', TokenKind::Constant),
        (b'.', TokenKind::Type),
    ],
    ..CSS
};
/// Python.
const PYTHON: Grammar = Grammar {
    line_comments: &["#"],
    block_comments: &[],
    quotes: &[
        Quote::lines("\"\"\"", "\"\"\""),
        Quote::lines("'''", "'''"),
        DOUBLE,
        SINGLE,
    ],
    keywords: tables::PYTHON_KEYWORDS,
    types: "int float str bool list dict set tuple bytes object",
    constants: "True False None",
    marks: DECORATOR,
    string_prefixes: &["r", "b", "f", "u", "rb", "br", "fr", "rf", "R", "B", "F", "U"],
    ..NAME
};
/// Ruby.
const RUBY: Grammar = Grammar {
    line_comments: &["#"],
    quotes: &[DOUBLE, SINGLE, Quote::lines("`", "`")],
    keywords: tables::RUBY_KEYWORDS,
    constants: "true false nil",
    marks: &[
        (b'@', TokenKind::Variable),
        (b'$', TokenKind::Variable),
        (b':', TokenKind::Constant),
    ],
    ..NAME
};
/// PHP.
const PHP: Grammar = Grammar {
    line_comments: &["//", "#"],
    quotes: &[Quote::lines("\"", "\""), Quote::lines("'", "'")],
    keywords: tables::PHP_KEYWORDS,
    constants: "true false null",
    marks: &[(b'$', TokenKind::Variable)],
    is_case_blind: true,
    ..C_FAMILY
};
/// Lua.
const LUA: Grammar = Grammar {
    line_comments: &["--"],
    block_comments: &[("--[[", "]]")],
    quotes: &[DOUBLE, SINGLE, Quote::lines("[[", "]]")],
    keywords: tables::LUA_KEYWORDS,
    constants: "true false nil",
    capital_types: false,
    ..NAME
};
/// Java.
const JAVA: Grammar = Grammar {
    quotes: &[Quote::lines("\"\"\"", "\"\"\""), DOUBLE, SINGLE],
    keywords: tables::JAVA_KEYWORDS,
    types: "boolean byte char double float int long short",
    constants: "true false null",
    marks: DECORATOR,
    ..C_FAMILY
};
/// Kotlin.
const KOTLIN: Grammar = Grammar {
    quotes: &[Quote::lines("\"\"\"", "\"\"\""), DOUBLE, SINGLE],
    keywords: tables::KOTLIN_KEYWORDS,
    constants: "true false null",
    marks: DECORATOR,
    ..C_FAMILY
};
/// Swift.
const SWIFT: Grammar = Grammar {
    quotes: &[Quote::lines("\"\"\"", "\"\"\""), DOUBLE],
    keywords: tables::SWIFT_KEYWORDS,
    constants: "true false nil",
    marks: &[(b'@', TokenKind::Attribute), (b'#', TokenKind::Keyword)],
    ..C_FAMILY
};
/// Dart.
const DART: Grammar = Grammar {
    quotes: &[
        Quote::lines("'''", "'''"),
        Quote::lines("\"\"\"", "\"\"\""),
        DOUBLE,
        SINGLE,
    ],
    keywords: tables::DART_KEYWORDS,
    types: "int double num bool",
    constants: "true false null",
    marks: DECORATOR,
    ..C_FAMILY
};
/// C#.
const CSHARP: Grammar = Grammar {
    keywords: tables::CSHARP_KEYWORDS,
    types: tables::CSHARP_TYPES,
    constants: "true false null",
    marks: &[(b'#', TokenKind::Keyword)],
    ..C_FAMILY
};
/// Go.
const GO: Grammar = Grammar {
    quotes: &[DOUBLE, SINGLE, Quote::raw_lines("`", "`")],
    keywords: tables::GO_KEYWORDS,
    types: tables::GO_TYPES,
    constants: "true false nil iota",
    ..C_FAMILY
};
/// Rust.
const RUST: Grammar = Grammar {
    quotes: &[Quote::raw_lines("r#\"", "\"#"), Quote::lines("\"", "\""), SINGLE],
    keywords: tables::RUST_KEYWORDS,
    types: tables::RUST_TYPES,
    constants: "true false",
    marks: &[(b'#', TokenKind::Attribute)],
    string_prefixes: &["r", "b", "br", "c"],
    has_lifetimes: true,
    has_macros: true,
    ..C_FAMILY
};
/// C.
const C: Grammar = Grammar {
    keywords: tables::C_KEYWORDS,
    types: tables::C_TYPES,
    constants: "NULL true false",
    marks: &[(b'#', TokenKind::Keyword)],
    ..C_FAMILY
};
/// C++.
const CPP: Grammar = Grammar {
    keywords: tables::CPP_KEYWORDS,
    constants: "NULL nullptr true false",
    ..C
};
/// YAML.
const YAML: Grammar = Grammar {
    line_comments: &["#"],
    quotes: &[DOUBLE, Quote::raw("'", "'")],
    constants: "true false null yes no on off ~",
    marks: &[
        (b'&', TokenKind::Variable),
        (b'*', TokenKind::Variable),
        (b'!', TokenKind::Type),
    ],
    keys_before_colons: true,
    has_dashes: true,
    capital_types: false,
    bare: TokenKind::String,
    ..NAME
};
/// TOML.
const TOML: Grammar = Grammar {
    line_comments: &["#"],
    quotes: &[
        Quote::lines("\"\"\"", "\"\"\""),
        Quote::raw_lines("'''", "'''"),
        DOUBLE,
        Quote::raw("'", "'"),
    ],
    constants: "true false",
    keys_before_equals: true,
    has_sections: true,
    has_dashes: true,
    capital_types: false,
    bare: TokenKind::Plain,
    ..NAME
};
/// INI.
const INI: Grammar = Grammar {
    line_comments: &[";", "#"],
    quotes: &[DOUBLE, Quote::raw("'", "'")],
    keys_before_equals: true,
    has_sections: true,
    has_dashes: true,
    capital_types: false,
    bare: TokenKind::String,
    ..NAME
};
/// Shell.
const SHELL: Grammar = Grammar {
    line_comments: &["#"],
    quotes: &[Quote::lines("\"", "\""), Quote::raw_lines("'", "'")],
    keywords: tables::SHELL_KEYWORDS,
    marks: &[(b'$', TokenKind::Variable)],
    has_dashes: true,
    capital_types: false,
    bare: TokenKind::Plain,
    ..NAME
};
/// PowerShell.
const POWERSHELL: Grammar = Grammar {
    line_comments: &["#"],
    block_comments: &[("<#", "#>")],
    quotes: &[
        Quote {
            escape: Some(b'`'),
            ..Quote::lines("\"", "\"")
        },
        Quote::raw_lines("'", "'"),
    ],
    keywords: tables::POWERSHELL_KEYWORDS,
    constants: "$true $false $null",
    marks: &[(b'$', TokenKind::Variable)],
    is_case_blind: true,
    has_dashes: true,
    has_cmdlets: true,
    capital_types: false,
    bare: TokenKind::Plain,
    ..NAME
};
/// SQL.
const SQL: Grammar = Grammar {
    line_comments: &["--"],
    block_comments: C_BLOCK,
    quotes: &[Quote::raw_lines("'", "'"), DOUBLE],
    keywords: tables::SQL_KEYWORDS,
    types: tables::SQL_TYPES,
    constants: "null true false",
    marks: &[(b'@', TokenKind::Variable)],
    is_case_blind: true,
    capital_types: false,
    bare: TokenKind::Plain,
    ..NAME
};
/// Dockerfile.
const DOCKERFILE: Grammar = Grammar {
    line_comments: &["#"],
    quotes: &[DOUBLE, SINGLE],
    keywords: tables::DOCKERFILE_KEYWORDS,
    marks: &[(b'$', TokenKind::Variable)],
    is_case_blind: true,
    has_dashes: true,
    capital_types: false,
    bare: TokenKind::Plain,
    ..NAME
};
