// Local theme whose HTML output has an empty `<body>`: it reads the
// resume but emits only a `metadata` value, which produces no markup.
// Used by `render_local_theme_with_empty_html_body_is_an_error` in
// `tests/render_cli.rs` (issue #320).

#let resume = json("/resume.json")
#metadata(resume)
