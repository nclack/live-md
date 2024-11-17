# 2024-11-16

## Plan

- [ ] update the flake to use nixpkgs master
- [ ] get ci more reliable
  - [x] increase test timeout
- [ ] optimize builds
- [/] optionally use the root README for the index

### Next

- [ ] dark mode
- [ ] command line options
- [ ] test that relative links between md files work correctly
- [ ] test that assets (images) work

## Notes

It does look like there's something racy. Every once in a while the page
renders blank and I need to trigger a re-render of the html then force a 
refresh. Happens more often then not.

### Using the README

I started down this road a bit. The idea is to use the README for the main
content but build a collapsible index from the contents of the doc folder.

I have a chat on claude with an initial design.

# 2024-11-15

## Plan

- [x] get test coverage up
- [x] build dependencies for ci
- [x] test timeouts

## Notes

When making the index page, I decided to take the file name and 
reformat that to create the listing in the generated `index.html`.

There are other possibilities here:
- Use the title heading from within the linked markdown file.
- Don't generate the index.html at all. Use a `README.md` or `toc`
  file if one exists.

Spent a long time screwing around with different tests etc.

Something wrong with watch tests on windows. Basically had to disable those.

# 2024-11-13

## Plan

- [x] running mvp
- [x] ci/cd
- [ ] get test coverage up

Next steps:
- dark mode
- command line options
- optionally use the root README for the index
- test that relative links between md files work correctly
- test that assets (images) work

## Notes

Generated the initial code via [claude.ai].

This was the initial prompt:

> [!note] Initial Prompt
> I'd like to create a simple http server that watches markdown files found
> in a directory tree, renders them to html, and serves those html files. If any
> markdown file changes, the html for that file should be regenerated and the
> browser page refreshed. The server should serve to localhost on a specified
> port, and when the server starts it should open an index page in a browser.
> 
> I want to write it rust. Which libraries should I use?
>
> Don't write code till I ask.

No index.html

> [!note] Initial Prompt
> the server appears to start, but I get a 404. I set the CONTENT_DIR to "doc"
> and I have a file "doc/devlog.md" that looks like it's transformed to "dist/
> doc/devlog.html" as expected. I'm not sure how the server knows what file to
> serve. It may need an index.html. Let's generate a default one that links to the
> rendered docs.

Some problem with relative paths.

There were a few things that needed to be cleaned and fixed up. `clippy`
basically provided the code suggestions for everything.

And now it seems to work! The live reload works too, lol.

Added CI from another rust project by copying over the `.github` and nix files.
No modifications needed.
