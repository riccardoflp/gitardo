# The operating systems

Everything Gitardo Desktop asks of the operating system goes through `crates/platform`, the only crate with `unsafe`.
This page lists what it asks, and how on each system. Windows comes first (see
[D6](08-decisions.md#d6--windows-first)); Linux and macOS follow behind the same interface.

## The interface

`platform` gives the rest of the program safe functions and types, the same on every system:

| Area | What the rest of the program sees |
| --- | --- |
| Windows | `Window::open(&event_loop, &options)`, title, size, position, scale factor, close request, dark or light title bar, `present(Frame, dirty_regions)`: a `Frame` is the pixels `0x00RRGGBB` of the renderer's buffer and its size, so that `platform` needs nothing of `render`. |
| Input | `InputEvent`: pointer moved, pressed, released (with the count of clicks), left, wheel (lines and pixels), key pressed and released with modifiers, text typed (after the input method), focus gained and lost, resized, scale changed, close requested, redraw requested, files dropped. `set_text_input_area` says where the input method shows its window. An `InputSender` feeds a window input as if the system sent it, for the tests of the application. |
| Cursor | Arrow, text beam, hand, resize east-west and north-south, grab, not allowed. |
| Event loop | One `EventLoop` for the UI thread and every window it opens; `run(handler)` waits for events, tagged with the window they are for, and wakes when another thread sends a message (`Waker`). Timers (`start_timer`, `stop_timer`) for blinking carets, delayed reads and animations. |
| Clipboard | Read and write text. |
| Files | Watching folders for changes (`Watcher`), the configuration and state folders of the user, the temporary folder, the trash. |
| Links | `make_folder_link(from, to)`: a junction on Windows, a symbolic link elsewhere; `unlink_folder_link`. |
| Shell | Open a URL in the browser, reveal a path in the file manager, open a path in the external editor or a terminal. |
| Dialogs | Choose a folder, choose a file to open, choose where to save. Everything else is drawn by Gitardo. |
| Credentials | Read, write and delete a secret by name in the system's credential store. |
| Fonts | `system_fonts()`: the paths of the system's font files, and the files of the default UI and monospace families. |
| Theme | Whether the system is dark or light, high contrast, the accent color, and a notification when it changes. |
| Processes | Running `git` uses `std::process`, which is enough; `platform` only adds killing a whole process tree on cancel. |

## Windows

The minimum is Windows 10 version 1809. The libraries are those every Windows has.

| Need | Calls |
| --- | --- |
| Window, messages | `user32`: `RegisterClassExW`, `CreateWindowExW`, `GetMessageW`/`PeekMessageW`, `DispatchMessageW`, `DefWindowProcW`, `PostMessageW` (to wake the loop), `SetTimer`. |
| HiDPI | `SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)`, `WM_DPICHANGED`, `GetDpiForWindow`. |
| Showing the frame | `gdi32`: `StretchDIBits` of a 32-bit top-down DIB into the window's DC on `WM_PAINT`, for the dirty regions only. |
| Keys and text | `WM_KEYDOWN`/`WM_SYSKEYDOWN` with `MapVirtualKeyW` for keys; `WM_CHAR` (UTF-16 surrogate pairs joined) for text; `imm32` (`ImmGetCompositionStringW`, `ImmSetCompositionWindow`) for input methods. |
| Pointer | `WM_MOUSEMOVE`, buttons, `WM_MOUSEWHEEL`/`WM_MOUSEHWHEEL`, `TrackMouseEvent` for leave, `SetCapture` while dragging. |
| Dark title bar | `dwmapi`: `DwmSetWindowAttribute(DWMWA_USE_IMMERSIVE_DARK_MODE)`. |
| Clipboard | `OpenClipboard`, `GetClipboardData(CF_UNICODETEXT)`, `SetClipboardData` with `GlobalAlloc`. |
| Watching files | `kernel32`: `ReadDirectoryChangesW` with `FILE_NOTIFY_CHANGE_*` on the repository and on its `.git` folder, in overlapped mode on the watcher thread. |
| Junctions | `CreateFileW` with `FILE_FLAG_OPEN_REPARSE_POINT` and `DeviceIoControl(FSCTL_SET_REPARSE_POINT)`; reading one with `FSCTL_GET_REPARSE_POINT`. Junctions need no rights, unlike symbolic links. |
| Open, reveal | `shell32`: `ShellExecuteW` for URLs and folders; `SHOpenFolderAndSelectItems` to reveal a file. |
| File dialogs | `comdlg32`: `GetOpenFileNameW`, `GetSaveFileNameW`; `shell32`: `SHBrowseForFolderW` for folders. |
| Credentials | `advapi32`: `CredReadW`, `CredWriteW`, `CredDeleteW`, `CredFree`, with `CRED_TYPE_GENERIC` and the target `Gitardo Desktop/<name>`. |
| Theme | Registry `HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize\AppsUseLightTheme` with `RegGetValueW`, `WM_SETTINGCHANGE` for changes; `SystemParametersInfoW(SPI_GETHIGHCONTRAST)`. |
| Fonts | `%WINDIR%\Fonts`: Segoe UI for the interface, Cascadia Mono or Consolas for code. |
| Killing a process tree | `CreateJobObjectW`, `AssignProcessToJobObject`, `TerminateJobObject`. |
| No console | `#![windows_subsystem = "windows"]` in the binary; `CREATE_NO_WINDOW` for git. |

## Linux

X11 first, which also runs under Wayland through XWayland. A Wayland backend is a later decision.

| Need | Calls |
| --- | --- |
| Window, events | `libX11`: `XOpenDisplay`, `XCreateWindow`, `XNextEvent`, `XPending`, `ConnectionNumber` with `poll` to wait on the display and a pipe that wakes the loop. |
| Showing the frame | `XPutImage`, or the MIT-SHM extension (`libXext`: `XShmPutImage`) when present. |
| HiDPI | `Xft.dpi` from `XResourceManagerString`. |
| Text and input methods | `XOpenIM`, `XCreateIC`, `Xutf8LookupString`. |
| Clipboard | The `CLIPBOARD` selection with `XConvertSelection`, `XSetSelectionOwner`, `SelectionRequest` events, `UTF8_STRING`. |
| Watching files | `inotify_init1`, `inotify_add_watch` from the C library, one watch per folder. |
| Links | `std::os::unix::fs::symlink`. |
| Open, reveal | `xdg-open` run as a process; revealing a file opens its folder. |
| File dialogs | Gitardo's own folder and file picker, drawn in the window. |
| Credentials | The Secret Service over D-Bus is a dependency in all but name; a file in the configuration folder readable only by the user (mode `0600`) is used instead, and the settings page says so. |
| Theme | `gsettings get org.gnome.desktop.interface color-scheme` run as a process when present; otherwise light. |
| Fonts | `/usr/share/fonts`, `/usr/local/share/fonts`, `~/.local/share/fonts`; DejaVu Sans and DejaVu Sans Mono, or Noto, as found. |

## macOS

The minimum is macOS 12. AppKit is reached through the Objective-C runtime (`libobjc`: `objc_getClass`,
`sel_registerName`, `objc_msgSend`), with each message wrapped in a typed function.

| Need | Calls |
| --- | --- |
| Application, window | `NSApplication`, `NSWindow`, a subclass of `NSView` made at run time with `objc_allocateClassPair` to receive events. |
| Showing the frame | A `CGImage` made from the buffer (`CoreGraphics`: `CGDataProviderCreateWithData`, `CGImageCreate`) set as the `contents` of the view's `CALayer`. |
| Keys and text | `keyDown:` with `interpretKeyEvents:` and the `NSTextInputClient` protocol for input methods. |
| Clipboard | `NSPasteboard generalPasteboard`, `NSPasteboardTypeString`. |
| Watching files | `CoreServices`: `FSEventStreamCreate` on the repository. |
| Open, reveal | `NSWorkspace openURL:`, `activateFileViewerSelectingURLs:`. |
| File dialogs | `NSOpenPanel`, `NSSavePanel`. |
| Credentials | `Security`: `SecItemAdd`, `SecItemCopyMatching`, `SecItemDelete` with `kSecClassGenericPassword`. |
| Theme | `NSApp effectiveAppearance`, and `AppleInterfaceThemeChangedNotification`. |
| Fonts | `/System/Library/Fonts`, `/Library/Fonts`, `~/Library/Fonts`; SF Pro for the interface, SF Mono or Menlo for code. |

## Text

Text is drawn by Gitardo itself, the same way on every system, so that the snapshots of the views are the same
everywhere.

- **Fonts**: TrueType outlines (`glyf`) and, after the first release, CFF outlines, read from the font files the
  platform names, with `cmap`, `hmtx`, `kern` and the pair adjustments of `GPOS` for spacing. Built from the
  OpenType specification.
- **Rasterizing**: coverage-based anti-aliasing of the outlines, with a cache of glyphs per size and scale. No hinting;
  grayscale, not subpixel.
- **Fallback**: a character the font lacks is looked for in the other fonts of the system, in a fixed order per system.
- **Out of scope for the first release**: right-to-left text, scripts that need shaping (Arabic, Indic), color emoji.
  Their characters are drawn one by one, or as a box when no font has them. The decision is
  [D4](08-decisions.md#d4--our-own-text-rendering).

## HTTP

Gitardo speaks HTTPS itself only to the API of GitHub and to the AI provider; git does its own networking. The decision
is [D13](08-decisions.md#d13--one-library-for-tls).

- **`crates/tls`** opens a TLS stream over `std::net::TcpStream` with `rustls` and `ring`, TLS 1.2 or 1.3, checking the
  server's certificate against the root certificates the system trusts, read by `rustls-native-certs` (the certificate
  store of Windows, the keychain of macOS, the certificate files of the Linux distribution). The rest of the program
  sees a stream that reads and writes, opened with a host name and a port, and an error in words when the certificate
  is not trusted.
- **`crates/http`** is Gitardo's own HTTP/1.1: a request with its headers and body, the answer's status, headers and
  body, `Content-Length` and chunked bodies, gzip not asked for, redirects followed up to 5 times to `https` only, and
  server-sent events read line by line as they arrive, for the streamed answers of a language model. One connection
  per request; timeouts for connecting (10 s) and between two reads (60 s, 300 s while a model streams).
- **Proxies**: `HTTPS_PROXY` (or `https_proxy`), and git's `http.proxy` read with `git config --get http.proxy`, are
  reached with `CONNECT`, with basic authentication when the proxy's URL has a user; `NO_PROXY` lists the hosts that
  are reached directly.
- **Secrets**: tokens and keys go in headers only, never in a URL, and never in the log, also when a request fails.
