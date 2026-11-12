# The operating systems

Everything Gitardo Desktop asks of the operating system goes through `crates/platform`, the only crate with `unsafe`.
This page lists what it asks, and how: through the libraries of the window for most of it, and on each system for the
rest. Windows comes first (see
[D6](08-decisions.md#d6--windows-first)); Linux and macOS follow behind the same interface.

## The interface

`platform` gives the rest of the program safe functions and types, the same on every system:

| Area | What the rest of the program sees |
| --- | --- |
| Windows | `Window::open(&context, &options)`, title, size, position, scale factor, close request, dark or light title bar, `present(Frame, dirty_regions)`: a `Frame` is the pixels `0x00RRGGBB` of the renderer's buffer and its size, so that `platform` needs nothing of `render`. |
| Input | `InputEvent`: pointer moved, pressed, released (with the count of clicks), left, wheel (lines and pixels), key pressed and released with modifiers, text typed (after the input method), focus gained and lost, resized, scale changed, close requested, redraw requested, files dropped. `set_text_input_area` says where the input method shows its window. An `InputSender` feeds a window input as if the system sent it, for the tests of the application. |
| Cursor | Arrow, text beam, hand, resize east-west and north-south, grab, not allowed. |
| Event loop | One `EventLoop` per program, on the UI thread, for every window it opens; `run(handler)` gives the handler a `Context` and each event: first `Event::Started`, where windows are opened, then the events of each window, tagged with it, and the messages another thread sends (`Waker`). The `Context` quits the loop and starts and stops timers (`start_timer`, `stop_timer`) for blinking carets, delayed reads and animations. |
| Clipboard | Read and write text. |
| Files | Watching folders for changes (`Watcher`), the configuration and state folders of the user, the temporary folder, the trash. |
| Links | `make_folder_link(from, to)`: a junction on Windows, a symbolic link elsewhere; `unlink_folder_link`. |
| Shell | Open a URL in the browser, reveal a path in the file manager, open a path in the external editor or a terminal. |
| Dialogs | Choose a folder, choose a file to open, choose where to save. Everything else is drawn by Gitardo. |
| Screen readers | The tree of what is on screen, built by `ui`, given to the system; what the screen reader asks for (a click, the focus) comes back as input. |
| Credentials | Read, write and delete a secret by name in the system's credential store. |
| Fonts | The fonts of the program are in `render`; `platform` gives the paths of the system's font files, for the fallback and for the settings `font.ui` and `font.code`. |
| Theme | Whether the system is dark or light, high contrast, and a notification when it changes. |
| Processes | Running `git` uses `std::process`, which is enough; `platform` only adds killing a whole process tree on cancel. |

## What the libraries do

The window and what comes with it are the same on every system, through the libraries of
[D16](08-decisions.md#d16--libraries-for-the-window-the-text-and-the-pixels), used by `platform` only:

| Need | Library |
| --- | --- |
| Windows, the event loop, waking it from another thread, timers | `winit`: one event loop for the UI thread and every window it opens; a user event wakes it. |
| Input: pointer, wheel, keys with modifiers, text typed, input methods | `winit`, turned into `InputEvent` of `platform`. |
| Scale (HiDPI), per monitor, and its changes | `winit`. |
| Cursors | `winit`. |
| Showing the frame | `softbuffer`: the pixels of the frame buffer copied into the window, for the regions that changed. |
| Dark or light title bar | `winit`, from the theme of Gitardo. |
| Whether the system is dark or light, and its changes | `winit`. |
| Clipboard | `arboard`. |
| File dialogs | `rfd`: the system's dialogs to choose a folder, a file to open, where to save. |
| Screen readers | `accesskit_winit`, with the tree of what is on screen that `ui` builds. |

## Windows

The minimum is Windows 10 version 1809. What the libraries do not cover is asked of the libraries every Windows has.

| Need | Calls |
| --- | --- |
| High contrast | `user32`: `SystemParametersInfoW(SPI_GETHIGHCONTRAST)`; `winit` says when the system's settings change. |
| Watching files | `kernel32`: `ReadDirectoryChangesW` with `FILE_NOTIFY_CHANGE_*` on the repository and on its `.git` folder, in overlapped mode on the watcher thread. |
| Junctions | `CreateFileW` with `FILE_FLAG_OPEN_REPARSE_POINT` and `DeviceIoControl(FSCTL_SET_REPARSE_POINT)`; reading one with `FSCTL_GET_REPARSE_POINT`. Junctions need no rights, unlike symbolic links. |
| Open, reveal | `shell32`: `ShellExecuteW` for URLs and folders; `SHOpenFolderAndSelectItems` to reveal a file. |
| Credentials | `advapi32`: `CredReadW`, `CredWriteW`, `CredDeleteW`, `CredFree`, with `CRED_TYPE_GENERIC` and the target `Gitardo Desktop/<name>`. |
| Killing a process tree | `CreateJobObjectW`, `AssignProcessToJobObject`, `TerminateJobObject`. |
| No console | `#![windows_subsystem = "windows"]` in the binary; `CREATE_NO_WINDOW` for git. |

## Linux

X11 and Wayland, both through `winit`.

| Need | Calls |
| --- | --- |
| High contrast | `gsettings get org.gnome.desktop.a11y.interface high-contrast` run as a process when present; otherwise not. |
| Watching files | `inotify_init1`, `inotify_add_watch` from the C library, one watch per folder. |
| Links | `std::os::unix::fs::symlink`. |
| Open, reveal | `xdg-open` run as a process; revealing a file opens its folder. |
| Credentials | The Secret Service over D-Bus, through a library ([D20](08-decisions.md#d20--libraries-where-they-serve)); until one is chosen, a file in the configuration folder readable only by the user (mode `0600`), and the settings page says so. |

## macOS

The minimum is macOS 12.

| Need | Calls |
| --- | --- |
| High contrast | `NSWorkspace accessibilityDisplayShouldIncreaseContrast`, through the Objective-C runtime (`libobjc`). |
| Watching files | `CoreServices`: `FSEventStreamCreate` on the repository. |
| Open, reveal | `open` and `open -R` run as a process. |
| Credentials | `Security`: `SecItemAdd`, `SecItemCopyMatching`, `SecItemDelete` with `kSecClassGenericPassword`. |

## Text

Text is drawn by `render` with `cosmic-text`, in the fonts that come with the program
([D18](08-decisions.md#d18--the-fonts-come-with-the-program)), the same way on every system, so that the snapshots of
the views are the same everywhere.

- **Fonts**: Inter for the interface, JetBrains Mono for hashes and code, in `crates/render/fonts/` with their licence.
  The settings `font.ui` and `font.code` may name a font of the system instead.
- **Shaping**: right-to-left text, scripts that need shaping, ligatures and emoji in color are drawn as they should be.
- **Fallback**: a character the fonts of the program lack is looked for in the fonts of the system. The snapshots of the
  tests look only in the fonts of the program, so that they do not depend on the machine.
- **Glyphs**: grayscale anti-aliasing, no subpixel colors, with a cache of glyphs per size and scale.

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
