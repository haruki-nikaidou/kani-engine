// @ts-check
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import markdoc from "@astrojs/markdoc";
import starlightThemeFlexoki from "starlight-theme-flexoki";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const kagGrammar = JSON.parse(
  readFileSync(
    fileURLToPath(new URL("./src/kag.tmLanguage.json", import.meta.url)),
    "utf-8",
  ),
);

const rhaiGrammar = JSON.parse(
  readFileSync(
    fileURLToPath(new URL("./src/rhai.tmLanguage.json", import.meta.url)),
    "utf-8",
  ),
);

// https://astro.build/config
export default defineConfig({
  site: "https://haruki-nikaidou.github.io/kani-engine",
  redirects: { "/": "/en" },
  integrations: [
    markdoc(),
    starlight({
      plugins: [starlightThemeFlexoki()],
      title: "Kani Game Engine",
      expressiveCode: {
        shiki: {
          langs: [kagGrammar, rhaiGrammar],
        },
      },
      defaultLocale: "en",
      locales: {
        en: { label: "English", lang: "en" },
        ja: { label: "日本語", lang: "ja" },
      },
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/haruki-nikaidou/kani-engine",
        },
      ],
      sidebar: [
        {
          label: "Start Here",
          translations: { ja: "はじめに" },
          items: [
            {
              label: "Getting Started",
              translations: { ja: "はじめかた" },
              slug: "start-here/getting-started",
            },
          ],
        },
        {
          label: "Guides",
          translations: { ja: "ガイド" },
          items: [
            {
              label: "Setup LSP",
              translations: { ja: "LSP のセットアップ" },
              slug: "guides/setup-lsp",
            },
          ],
        },
        {
          label: "Reference",
          translations: { ja: "リファレンス" },
          autogenerate: { directory: "reference" },
        },
      ],
    }),
  ],
});
