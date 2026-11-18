# The operating systems

Everything Gitardo Desktop asks of the operating system goes through `crates/platform`, and what the engine asks of it
without a window (the credential store, links to folders) through `crates/system`, which `platform` uses for the
bindings they share. They are the only crates with `unsafe`. This page lists what it asks, and how: through the
libraries of the window for most of it, and on each system for the rest. Windows comes first (see
[D6](../08-decisions.md#d6--windows-first)); Linux and macOS follow behind the same interface.

## The interface

`platform` gives the rest of the program safe functions and types, the same on every system:

| Area | What the rest of the program sees |
| --- | --- |
| Windows | `Window::open(&context, &options)`, title, size, position, scale factor, close request, dark or light title bar, `present(Frame, dirty_regions)`: a `Frame` is the pixels `0x00RRGGBB` of the renderer's buffer and its size, so that `platform` needs nothing of `render`. Minimize, maximize, restore and whether it is maximized. |
| Title bar drawn by the program | `Decorations::Drawn` in the options of a window: the program draws its title bar ([D58](../08-decisions.md#d58--the-window-draws-its-own-title-bar-with-the-logo-and-the-name-of-gitardo)). `set_frame_parts(function)` says what each place of the window is (`FramePart`: the inside, the caption, the three buttons, an edge or a corner), from a place and the size of the window in logical pixels; the window calls it whenever the system asks, between frames. `InputEvent::MoveStarted` says the window starts to be moved or resized. `placement()` says where the window is and how large as it is restored from maximized or minimized, and whether it is maximized; `WindowOptions::position` and `is_maximized` open it so, a position on no screen being left to the system. `title()` reads the title back, which the taskbar says though it is not drawn. |
| Input | `InputEvent`: pointer moved, pressed, released (with the count of clicks), left, wheel (lines and pixels), key pressed and released with modifiers, text typed (after the input method), focus gained and lost, resized, scale changed, close requested, redraw requested, files dropped. `set_text_input_area` says where the input method shows its window. An `InputSender` feeds a window input as if the system sent it, for the tests of the application. |
| Cursor | Arrow, text beam, hand, resize east-west and north-south, grab, not allowed. |
| Event loop | One `EventLoop` per program, on the UI thread, for every window it opens; `run(handler)` gives the handler a `Context` and each event: first `Event::Started`, where windows are opened, then the events of each window, tagged with it, and the messages another thread sends (`Waker`). The `Context` quits the loop and starts and stops timers (`start_timer`, `stop_timer`) for blinking carets, delayed reads and animations. |
| Clipboard | Read and write text. |
| Files | Watching folders for changes (`Watcher`), the configuration and state folders of the user, the temporary folder, the trash. |
| Links | `link_folder(link, target)`: a junction on Windows, a symbolic link elsewhere; in `system`, for the engine. |
| Shell | Open a URL in the browser, reveal a path in the file manager, open a path in the external editor or a terminal. |
| Dialogs | Choose a folder, choose a file to open, choose where to save. Everything else is drawn by Gitardo. |
| Screen readers | The tree of what is on screen, built by `ui`, given to the system; what the screen reader asks for (a click, the focus) comes back as input. |
| Credentials | Read, write and delete a secret by name in the system's credential store: in `system`, for the engine. |
| Fonts | The fonts of the program are in `render`; `platform` gives the paths of the system's font files, for the fallback and for the settings `font.ui` and `font.code`. |
| Theme | Whether the system is dark or light, high contrast, and a notification when it changes. |
| Processes | Running `git` uses `std::process`, which is enough; `platform` only adds killing a whole process tree on cancel. `ProcessTree::of(process)` holds a task and whatever it starts, to end them together (`tasks.stop`): a job object on Windows, the process group of the task elsewhere; in `system`. |

## What the libraries do

The window and what comes with it are the same on every system, through the libraries of
[D16](../08-decisions.md#d16--libraries-for-the-window-the-text-and-the-pixels), used by `platform` only:

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
| Junctions | `CreateFileW` with `FILE_FLAG_OPEN_REPARSE_POINT` and `DeviceIoControl(FSCTL_SET_REPARSE_POINT)`. Junctions need no rights, unlike symbolic links. |
| Open, reveal | `shell32`: `ShellExecuteW` for URLs and folders; `SHOpenFolderAndSelectItems` to reveal a file. |
| Credentials | `advapi32`: `CredReadW`, `CredWriteW`, `CredDeleteW`, `CredFree`, with `CRED_TYPE_GENERIC` and the target `Gitardo Desktop/<name>`. |
| Killing a process tree | `CreateJobObjectW`, `AssignProcessToJobObject`, `TerminateJobObject`. |
| No console | `#![windows_subsystem = "windows"]` in the binary; `CREATE_NO_WINDOW` for git. |
| Title bar drawn by the program | `winit` opens the window without decorations and with the shadow of its frame (`with_undecorated_shadow`): it keeps the styles of a window with a frame (`WS_CAPTION`, `WS_SIZEBOX`, `WS_SYSMENU`, the boxes of the buttons), so that the system keeps its shadow, its rounded corners, Aero Snap and its keys, and lays the inside over the frame (`WM_NCCALCSIZE`), within the work area of its screen while it is maximized. `comctl32`: `SetWindowSubclass`, `DefSubclassProc` and `RemoveWindowSubclass` put a procedure in front of the one of `winit`, taken away before the window is destroyed or when it is. It answers `WM_NCHITTEST` from the function of `set_frame_parts`: `HTCAPTION`, which the system moves the window from, maximizes and restores it from with a double click and opens its menu from with a right click; `HTMAXBUTTON`, which shows Snap Layouts on Windows 11; `HTLEFT` to `HTBOTTOMRIGHT` for the edges and corners; `HTCLIENT` for the rest, **Minimize** and **Close** included. `WM_NCMOUSEMOVE`, `WM_NCMOUSELEAVE` (asked with `TrackMouseEvent`), `WM_NCLBUTTONDOWN`, `WM_NCLBUTTONDBLCLK` and `WM_NCLBUTTONUP` over the maximize button are handed to the program as input, at the place `ScreenToClient`, `GetClientRect` and `GetDpiForWindow` give. `WM_SYSCHAR` of a space goes to `DefWindowProcW`, which opens the system's menu for `Alt+Space`, since `winit` keeps it from the system. `WM_ENTERSIZEMOVE` is handed to the program as `MoveStarted`. `IsZoomed` and `IsIconic` say whether the window is restored as it moves, before `winit` knows of a maximized window from its new size. |

## Linux

X11 and Wayland, both through `winit`, which picks Wayland when `WAYLAND_DISPLAY` is set and X11 otherwise. What the
libraries do not cover is in `crates/platform/src/linux/`; the C library is declared there by hand, as the libraries of
Windows are, and nothing else is linked.

| Need | Calls |
| --- | --- |
| High contrast | `gsettings get org.gnome.desktop.a11y.interface high-contrast` run as a process when present; otherwise not. Read again whenever a window gets the keyboard back, as on Windows. |
| Double click | `gsettings get org.gnome.desktop.peripherals.mouse double-click`, read once; 400 ms when absent. The distance is the usual 4 pixels, at the scale of the window. |
| Watching files | `inotify_init1`, `inotify_add_watch`, `inotify_rm_watch` and `poll` from the C library, one watch per folder: every folder under the one watched when it opens, and each folder created or moved in afterwards, whose contents are then said as changed. Links to folders are not followed. A folder over the system's limit of watches (`fs.inotify.max_user_watches`) is not watched. |
| Links | `std::os::unix::fs::symlink`. |
| Local time | `localtime_r` of the C library, whose `tm_gmtoff` is how far the local time is from UTC (`${date}` in the names of new branches). |
| Open, reveal | `xdg-open` run as a process; revealing a file opens its folder. |
| Credentials | The Secret Service over D-Bus, through a library ([D22](../08-decisions.md#d22--libraries-with-judgment)); until one is chosen, the file `credentials` in the configuration folder (`$XDG_CONFIG_HOME/gitardo-desktop`, or `~/.config/gitardo-desktop`), readable only by the user (mode `0600`, its folder `0700` when Gitardo makes it), and the settings page says so. |
| Clipboard | `arboard` over X11; on Wayland through XWayland, since its Wayland part is not built. |
| File dialogs | `rfd` through the desktop's portal (`xdg-desktop-portal`), which falls back to `zenity` when there is no portal. |
| Fonts of the system | `fontdb`, through `cosmic-text`, reads the usual folders (`/usr/share/fonts`, `/usr/local/share/fonts`, `~/.local/share/fonts`, `~/.fonts`), not the configuration of fontconfig. |
| Terminal, external editor | The terminal is `x-terminal-emulator` started in the folder, unless `terminal.external` says another; the editor is `editor.external`. |
| Title bar drawn by the program | `winit` opens the window without decorations. A press of the main button on a place `set_frame_parts` says is the caption asks the compositor to move the window (`drag_window`), twice to maximize or restore it; a press on an edge or a corner asks it to resize the window (`drag_resize_window`), whose shape the pointer shows over them; a press of the other button on the caption asks for the menu of the window (`show_window_menu`), which `winit` 0.30 does not open on X11 or Wayland yet. A move or a resize asked is handed to the program as `MoveStarted`. The press is the system's then, and does not reach the program. |

## macOS

The minimum is macOS 12. What the libraries do not cover is in `crates/platform/src/macos/`, one file for each library
of the system it declares by hand (`CoreFoundation`, `CoreServices`, `Security`, `libobjc`, `libSystem`), as on the
other systems.

| Need | Calls |
| --- | --- |
| High contrast | `[[NSWorkspace sharedWorkspace] accessibilityDisplayShouldIncreaseContrast]`, through the Objective-C runtime (`objc_getClass`, `sel_registerName`, `objc_msgSend`). Read again whenever a window gets the keyboard back, as on the other systems. |
| Double click | `[NSEvent doubleClickInterval]`, read once; 500 ms when it says nothing usable. The distance is the usual 4 pixels, at the scale of the window. |
| Watching files | `CoreServices`: `FSEventStreamCreate` on each folder, with `kFSEventStreamCreateFlagFileEvents`, `NoDefer` and `WatchRoot`, a latency of 50 ms, on a queue of its own (`dispatch_queue_create`, `FSEventStreamSetDispatchQueue`). The paths are said from the folder as the system writes it, with its links resolved; changes lost (`MustScanSubDirs`, `UserDropped`, `KernelDropped`) are an overflow, and `RootChanged` says the folder is gone. |
| Links | `std::os::unix::fs::symlink`. |
| Local time | `localtime_r` of the C library, whose `tm_gmtoff` is how far the local time is from UTC. |
| Open, reveal | `open` and `open -R` run as a process. |
| Credentials | `Security`: `SecItemCopyMatching`, `SecItemUpdate`, `SecItemAdd`, `SecItemDelete` with `kSecClassGenericPassword`, the service `Gitardo Desktop` and the name of the secret as the account. |
| Clipboard, file dialogs | `arboard` and `rfd`, through `AppKit`. |
| Terminal, external editor | The terminal is `Terminal.app` opened on the folder (`open -a Terminal`), unless `terminal.external` says another; the editor is `editor.external`. |
| Title bar drawn by the program | `winit` makes the system's title bar transparent (`with_titlebar_transparent`), the inside of the window reaching the top (`with_fullsize_content_view`), and hides the title (`with_title_hidden`): the system keeps its three buttons at the top left, its frame and its edges. A press of the main button on the caption asks the system to move the window (`drag_window`), twice to zoom it, as on Linux; the system resizes the window from its edges itself. |

## Text

Text is drawn by `render` with `cosmic-text`, in the fonts that come with the program
([D18](../08-decisions.md#d18--the-fonts-come-with-the-program)), the same way on every system, so that the snapshots of
the views are the same everywhere.

- **Fonts**: Inter for the interface, JetBrains Mono for hashes and code, in `crates/render/fonts/` with their licence.
  The settings `font.ui` and `font.code` may name a font of the system instead.
- **Shaping**: right-to-left text, scripts that need shaping, ligatures and emoji in color are drawn as they should be.
- **Fallback**: a character the fonts of the program lack is looked for in the fonts of the system. The snapshots of the
  tests look only in the fonts of the program, so that they do not depend on the machine.
- **Glyphs**: grayscale anti-aliasing, no subpixel colors, with a cache of glyphs per size and scale.

## HTTP

Gitardo speaks HTTPS itself only to the API of GitHub and to the AI provider; git does its own networking. The decision
is [D13](../08-decisions.md#d13--one-library-for-tls).

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
