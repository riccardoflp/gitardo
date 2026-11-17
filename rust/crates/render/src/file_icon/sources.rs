//! The SVG files of the icons of Symbols, kept in `icons/symbols/` with its licence (MIT): those of files, then
//! those of folders, each with the name of its file, in the order of their names.

/// The name of each icon of a folder of `icons/symbols/` with the text of its SVG file.
macro_rules! icons {
    ($folder:literal: $($name:literal),* $(,)?) => {
        [$(($name, include_str!(concat!("../../icons/symbols/", $folder, "/", $name, ".svg")))),*]
    };
}

/// The icons of files.
pub(super) const FILES: [(&str, &str); 261] = icons!("files":
    "angular", "angular-component", "angular-directive", "angular-guard", "angular-module", "angular-pipe",
    "angular-service", "animate", "astro", "audio", "babel", "biome", "brackets-blue", "brackets-gray",
    "brackets-green", "brackets-orange", "brackets-pink", "brackets-purple", "brackets-red", "brackets-sky",
    "brackets-yellow", "browserslist", "bruno", "bun", "c", "caddy", "capacitor", "clapper", "claude", "clojure",
    "cloudflare-workers", "cmake", "code-blue", "code-gray", "code-green", "code-orange", "code-pink", "code-purple",
    "code-red", "code-sky", "code-yellow", "coffeescript", "coldfusion", "compressed", "contentlayer", "cplus",
    "crystal", "csharp", "csv", "cucumber", "cuda", "cursor", "cypress", "dart", "database", "deno", "docker",
    "docker-green", "docker-orange", "docker-pink", "docker-purple", "docker-red", "docker-yellow", "document",
    "docusaurus", "drawio", "drizzle", "dts", "dune", "earthfile", "editorconfig", "elixir", "erlang", "eslint", "exe",
    "expressive-code", "firebase", "font", "fortran", "fresh", "fsharp", "func", "gatsby", "gdextension", "gdignore",
    "gdproject", "gdscript", "gdshader", "gear", "gif", "git", "github", "gitlab", "gleam", "globe", "go", "go-green",
    "go-orange", "go-pink", "go-purple", "go-red", "go-yellow", "gradle", "graphql", "gulp", "h", "haml", "haskell",
    "http", "hugo", "i18n", "ignore", "illustrator", "image", "ionic", "java", "java-class", "jenkins", "jest", "js",
    "js-test", "julia", "julia-markdown", "keystatic", "knip", "kotlin", "kubb", "laravel", "license", "link", "liquid",
    "lock", "lua", "luau", "lunaria", "markdoc", "markdown", "maven", "mdx", "minecraft", "mongo", "moon", "nest",
    "nest-controller", "nest-decorator", "nest-guard", "nest-middleware", "nest-service", "netlify", "next", "nim",
    "nix", "node", "nodemon", "notebook", "npm", "nunjucks", "nuxt", "nx", "ocaml", "orval", "oxc", "panda", "patch",
    "pdf", "perl", "photoshop", "php", "pkl", "pnpm", "postcss", "prettier", "prisma", "proto", "pug", "pulumi",
    "puppeteer", "puzzle", "python", "r", "razor", "react", "react-test", "react-ts", "readme", "redux-actions",
    "redux-effects", "redux-facade", "redux-reducer", "redux-selector", "renovate", "rescript", "rescript-interface",
    "resource", "resource-sky", "robot", "rolldown", "rome", "rsbuild", "rslib", "rspack", "ruby", "rust", "sanity",
    "sass", "sbt", "scala", "severless", "shadcn", "shell", "slint", "solidity", "statamic-antlers", "storybook",
    "stylelint", "stylus", "supabase", "svelte", "svelte-service", "svelte-ts", "svg", "svx", "swc", "swift",
    "tailwind", "taskfile", "tauri", "terraform", "tex", "text", "ts", "ts-test", "ts-types", "tsconfig", "turborepo",
    "twig", "typst", "unity-asmdef", "unity-asmref", "unity-asset", "unity-meta", "unity-package", "unity-scene",
    "unity-shader", "unocss", "uv", "v", "vanilla-extract", "vercel", "video", "visual-studio", "vite", "vitest", "vue",
    "webgpu", "webpack", "xml", "yaml", "yarn", "yummacss", "zig",
);

/// The icons of folders.
pub(super) const FOLDERS: [(&str, &str); 104] = icons!("folders":
    "folder", "folder-actions", "folder-android", "folder-angular", "folder-app", "folder-assets", "folder-aws",
    "folder-azure", "folder-blue", "folder-blue-code", "folder-blue-outline", "folder-bruno", "folder-build",
    "folder-changeset", "folder-claude", "folder-components", "folder-config", "folder-constants", "folder-context",
    "folder-core", "folder-cursor", "folder-cypress", "folder-database", "folder-docker", "folder-documents",
    "folder-drizzle", "folder-effects", "folder-expo", "folder-facade", "folder-firebase", "folder-fonts",
    "folder-github", "folder-gitlab", "folder-globe", "folder-go", "folder-godot", "folder-gradle", "folder-graphql",
    "folder-gray", "folder-gray-code", "folder-gray-outline", "folder-green", "folder-green-code",
    "folder-green-outline", "folder-helpers", "folder-hooks", "folder-i18n", "folder-images", "folder-interceptors",
    "folder-interfaces", "folder-ios", "folder-java", "folder-js", "folder-kadence", "folder-layout", "folder-lock",
    "folder-mail", "folder-maven", "folder-middleware", "folder-models", "folder-modules", "folder-mongo",
    "folder-moon", "folder-netlify", "folder-next", "folder-nginx", "folder-node-modules", "folder-orange",
    "folder-orange-code", "folder-orange-outline", "folder-pink", "folder-pink-code", "folder-pink-outline",
    "folder-pipes", "folder-prisma", "folder-providers", "folder-purple", "folder-purple-code", "folder-purple-outline",
    "folder-react", "folder-red", "folder-red-code", "folder-red-outline", "folder-redis", "folder-reducer",
    "folder-router", "folder-sass", "folder-selector", "folder-services", "folder-shared", "folder-sky",
    "folder-sky-code", "folder-sky-outline", "folder-supabase", "folder-target", "folder-tauri", "folder-tina",
    "folder-utils", "folder-vercel", "folder-vscode", "folder-yellow", "folder-yellow-code", "folder-yellow-outline",
    "folder-zed",
);
