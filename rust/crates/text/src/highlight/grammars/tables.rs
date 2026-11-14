//! The keywords and the names of types of the languages, as their references list them, each list one string of words
//! between spaces.

/// The keywords of TypeScript, JavaScript's among them.
pub(super) const TYPESCRIPT_KEYWORDS: &str = "\
    abstract as async await break case catch class const continue debugger declare default delete do \
    else enum export extends finally for from function get if implements import in infer instanceof \
    interface is keyof let module namespace new of private protected public readonly return \
    satisfies set static super switch this throw try type typeof var void while with yield";
/// The names of the types of TypeScript.
pub(super) const TYPESCRIPT_TYPES: &str = "any bigint boolean never number object string symbol unknown";
/// The constants of JavaScript.
pub(super) const JAVASCRIPT_CONSTANTS: &str = "true false null undefined NaN Infinity";
/// The keywords of Python.
pub(super) const PYTHON_KEYWORDS: &str = "\
    and as assert async await break case class continue def del elif else except finally for from \
    global if import in is lambda match nonlocal not or pass raise return try while with yield";
/// The keywords of Ruby.
pub(super) const RUBY_KEYWORDS: &str = "\
    alias and begin break case class def defined? do else elsif end ensure for if in module next not \
    or redo rescue retry return self super then undef unless until when while yield";
/// The keywords of PHP.
pub(super) const PHP_KEYWORDS: &str = "\
    abstract and array as break callable case catch class clone const continue declare default do \
    echo else elseif empty enddeclare endfor endforeach endif endswitch endwhile extends final \
    finally fn for foreach function global goto if implements include include_once instanceof \
    insteadof interface isset list match namespace new or print private protected public readonly \
    require require_once return static switch throw trait try unset use var while xor yield";
/// The keywords of Lua.
pub(super) const LUA_KEYWORDS: &str = "\
    and break do else elseif end for function goto if in local not or repeat return then until while";
/// The keywords of Java.
pub(super) const JAVA_KEYWORDS: &str = "\
    abstract assert break case catch class const continue default do else enum extends final finally \
    for goto if implements import instanceof interface native new package permits private protected \
    public record return sealed static strictfp super switch synchronized this throw throws \
    transient try var void volatile while yield";
/// The keywords of Kotlin.
pub(super) const KOTLIN_KEYWORDS: &str = "\
    abstract annotation as break by catch class companion const constructor continue crossinline \
    data do else enum expect external final finally for fun get if import in infix init inline inner \
    interface internal is lateinit noinline object open operator out override package private \
    protected public reified return sealed set super suspend tailrec this throw try typealias val \
    var vararg when where while";
/// The keywords of Swift.
pub(super) const SWIFT_KEYWORDS: &str = "\
    any as associatedtype async await break case catch class continue default defer deinit do else \
    enum extension fallthrough fileprivate for func guard if import in init inout internal is let \
    open operator private protocol public repeat rethrows return self Self some static struct \
    subscript super switch throw throws try typealias var where while";
/// The keywords of Dart.
pub(super) const DART_KEYWORDS: &str = "\
    abstract as assert async await break case catch class const continue covariant default deferred \
    do dynamic else enum export extends extension external factory final finally for get hide if \
    implements import in interface is late library mixin new on operator part required rethrow \
    return set show static super switch sync this throw try typedef var void while with yield";
/// The keywords of C#.
pub(super) const CSHARP_KEYWORDS: &str = "\
    abstract as async await base break case catch checked class const continue default delegate do \
    else enum event explicit extern finally fixed for foreach get goto if implicit in init interface \
    internal is lock namespace new operator out override params private protected public readonly \
    record ref return sealed set sizeof stackalloc static struct switch this throw try typeof \
    unchecked unsafe using value var virtual void volatile while yield";
/// The names of the types of C#.
pub(super) const CSHARP_TYPES: &str = "\
    bool byte char decimal double float int long object sbyte short string uint ulong ushort";
/// The keywords of Go.
pub(super) const GO_KEYWORDS: &str = "\
    break case chan const continue default defer else fallthrough for func go goto if import \
    interface map package range return select struct switch type var";
/// The names of the types of Go.
pub(super) const GO_TYPES: &str = "\
    any bool byte complex64 complex128 error float32 float64 int int8 int16 int32 int64 rune string \
    uint uint8 uint16 uint32 uint64 uintptr";
/// The keywords of Rust.
pub(super) const RUST_KEYWORDS: &str = "\
    as async await break const continue crate dyn else enum extern fn for if impl in let loop match \
    mod move mut pub ref return self Self static struct super trait type unsafe use where while \
    yield";
/// The names of the types of Rust that do not start with a capital letter.
pub(super) const RUST_TYPES: &str = "bool char f32 f64 i8 i16 i32 i64 i128 isize str u8 u16 u32 u64 u128 usize";
/// The keywords of C.
pub(super) const C_KEYWORDS: &str = "\
    auto break case const continue default do else enum extern for goto if inline register restrict \
    return sizeof static struct switch typedef union volatile while";
/// The names of the types of C.
pub(super) const C_TYPES: &str = "bool char double float int long short signed size_t unsigned void wchar_t";
/// The keywords of C++, C's among them.
pub(super) const CPP_KEYWORDS: &str = "\
    alignas alignof asm auto break case catch class co_await co_return co_yield concept const \
    const_cast consteval constexpr constinit continue decltype default delete do dynamic_cast else \
    enum explicit export extern for friend goto if inline mutable namespace new noexcept operator \
    private protected public register reinterpret_cast requires return sizeof static static_assert \
    static_cast struct switch template this thread_local throw try typedef typeid typename union \
    using virtual volatile while";
/// The keywords of the shells.
pub(super) const SHELL_KEYWORDS: &str = "\
    alias break case continue declare do done elif else esac exit export fi for function if in local \
    readonly return select shift source then until unset while";
/// The keywords of PowerShell.
pub(super) const POWERSHELL_KEYWORDS: &str = "\
    begin break catch class continue data do dynamicparam else elseif end enum exit filter finally \
    for foreach function if in param process return switch throw trap try until using while";
/// The keywords of SQL.
pub(super) const SQL_KEYWORDS: &str = "\
    add all alter and as asc begin between by case check column commit constraint create default \
    delete desc distinct drop else end exists foreign from full group having in index inner insert \
    into is join key left like limit not offset on or order outer primary references returning right \
    rollback select set table then transaction union unique update values view when where with";
/// The names of the types of SQL.
pub(super) const SQL_TYPES: &str = "\
    bigint boolean char date decimal float int integer numeric real serial smallint text timestamp \
    varchar";
/// The instructions of a Dockerfile.
pub(super) const DOCKERFILE_KEYWORDS: &str = "\
    add arg as cmd copy entrypoint env expose from healthcheck label maintainer onbuild run shell \
    stopsignal user volume workdir";
