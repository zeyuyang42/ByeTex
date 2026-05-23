# Vendored: tree-sitter-latex

Source: `@pfoerster/tree-sitter-latex` npm package, version **0.6.0** (Sept 2025).

Upstream: https://github.com/latex-lsp/tree-sitter-latex

These files (`src/parser.c`, `src/scanner.c`, `src/tree_sitter/*.h`,
`src/node-types.json`, `src/grammar.json`) are the generated outputs of running
`tree-sitter generate` on `grammar.js` from the upstream repo. We vendor them to
avoid requiring the `tree-sitter` CLI as a build dependency.

## License

MIT — see `LICENSE` in this directory.

## Updating

To pull a newer version:

```bash
npm pack @pfoerster/tree-sitter-latex
tar xzf pfoerster-tree-sitter-latex-*.tgz
cp -r package/src/* ./src/
cp package/LICENSE ./LICENSE
```

Then bump the version note in this README.
