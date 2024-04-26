use pulldown_cmark::Parser;
use pulldown_cmark::{CodeBlockKind, HeadingLevel, Tag, TagEnd};
use pulldown_cmark::{CowStr, Event};
use std::fs::File;
use std::io::prelude::*;
use std::process::{exit, Command};

/*
* Currently incredibly cursed since a large portion was written at 3am
* */

const MATH_DIR: &str = "math/";
const CACHE_MATH: bool = false;

const START_ARTICLE: &str = r#"<!doctype html>
<html lang="en-AU">

<head>
	<link rel="icon" href="data:image/gif;base64,R0lGODlhAQABAAAAACwAAAAAAQABAAA=">
        <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/default.min.css">
<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/highlight.min.js"></script>

<!-- and it's easy to individually load additional languages -->
<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/languages/go.min.js"></script>

<script>hljs.highlightAll();</script>
	<meta charset="utf-8" />
	<style>
		:root {
			font: 1.15rem -apple-system, BlinkMacSystemFont, avenir next, avenir, helvetica, helvetica neue, ubuntu, roboto, noto, segoe ui, arial, sans-serif
		}

		.typst-doc {
			vertical-align: middle;
		}

		body {
			max-width: 70ch;
			margin-left: auto;
			margin-right: auto;
		}

		header {
			padding-top: 1rem;
		}

		header li {
			float: right;
		}

		header li a,
		header li button {
			display: block;
			text-align: center;
			text-decoration: none;
			padding-top: 1rem;
			padding-bottom: 0.2rem;
			padding-left: 0.5rem;
		}

		ul {
			overflow: hidden;
			padding: 0;
			margin: 0;
		}

		.hr-list {
			margin-left: 0.5rem;
			margin-right: 0.5rem;
			border: 0;
			flex: 1 0 1rem;
		}

		body li {
			display: flex;
			padding-bottom: 0.2rem;
		}

		a {
			color: inherit;
		}

		.dark-mode {
			background-color: black;
			color: white;
                        fill: white;
		}
.math-inline{vertical-align: middle;overflow: visible} .math{overflow: visible; width: 100%} img{width:100%}
	</style>
	<script>
		function darkToggle() {
			var element = document.body;
			element.classList.toggle("dark-mode");
		}
	</script>
	<meta name="viewport" content="width=device-width" />
	<title>Home</title>
</head>

<body class="dark-mode">
	<header>
		<ul>
			<li style="float: left;"><a style="padding-left:0;" href="index.html"><b>Home</b></a>
			</li>
			<li><button onclick="darkToggle()"
					style="border:none;background-color:inherit; color: inherit; font-size: inherit; font: inherit;">💡</button>
			</li>
			<li><a href="about.html">About</a></li>
		</ul>
		<hr style="margin-bottom: 1.5rem">
	</header>
"#;

const END_ARTICLE: &str = r#"</body></html>"#;

fn math_to_svg(math: &str, output_svg: String) {
    let get_temp_file = || {
        let filename = Command::new("mktemp").output().unwrap();
        let filename = String::from_utf8(filename.stdout).unwrap();
        filename.trim().to_owned()
    };

    let typst_file = get_temp_file();
    let intermediate_svg = format!("{}.svg", get_temp_file());

    // typst requires a file as it cannot take input from stdin
    let mut file = File::create(&typst_file).unwrap();
    file.write_all(format!("#show math.equation: set text(1.15em)\n{math}").as_bytes())
        .unwrap();

    // generate the intermediate svg on a full page
    let typst = Command::new("typst")
        .args(["compile", &typst_file, &intermediate_svg])
        .output()
        .unwrap();
    if !typst.status.success() {
        log::error!("typst exited with: {:?}", String::from_utf8(typst.stderr));
        exit(typst.status.code().unwrap());
    }

    // get rid of whitespace in the intermediate svg and output to "MATH_DIR"
    let inkscape = Command::new("inkscape")
        .args([
            &format!("--export-filename={output_svg}"),
            "--export-area-drawing",
            &intermediate_svg,
        ])
        .output()
        .unwrap();

    if !inkscape.status.success() {
        log::error!(
            "inkscape exited with: {:?}",
            String::from_utf8(inkscape.stderr)
        );
        exit(inkscape.status.code().unwrap());
    }

    let svgcleaner = Command::new("svgcleaner")
        .args([
            "--trim-ids=no",
            "--coordinates-precision=2",
            "--properties-precision=2",
            "--transforms-precision=2",
            "--paths-coordinates-precision=2",
            &output_svg,
            &output_svg,
        ])
        .output()
        .unwrap();
    if !svgcleaner.status.success() {
        log::error!(
            "svgcleaner exited with: {:?}",
            String::from_utf8(svgcleaner.stderr)
        );
        exit(svgcleaner.status.code().unwrap());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OurTag {
    H1,
    Code,
    CodeInline,
    P,
    Math,
    InlineMath,
}

fn parse_article(file: &str) -> String {
    let mut file = File::open(file).unwrap();
    let mut input = String::new();
    file.read_to_string(&mut input).unwrap();

    let mut tagstack = Vec::new();
    let mut output: String = START_ARTICLE.to_owned();

    let parser = Parser::new(&input);

    for event in parser {
        match event {
            Event::Start(t) => start_tag(&mut tagstack, t, &mut output),
            Event::End(t) => end_tag(&mut tagstack, t, &mut output),
            Event::Text(t) => text(t, &mut output),
            _ => {}
        }
    }

    output += END_ARTICLE;

    output
}

fn render_math(math: String, is_inline: bool) -> String {
    let output_svg = format!("{MATH_DIR}{math}.svg");
    // only render if svg doesn't already exist
    if std::fs::metadata(&output_svg).is_err() || !CACHE_MATH {
        math_to_svg(&math, output_svg.clone());
    }
    let out = std::fs::read_to_string(output_svg).unwrap();

    if is_inline {
        out.replacen("svg", "svg class=\"math-inline\"", 1)
    } else {
        out.replacen("svg", "svg class=\"math\"", 1)
    }
}

fn text(text: CowStr, output: &mut String) {
    // \$ is replaced by $ when outside of a math block
    // $ denotes a math block
    // it is an inline math block if there are no
    // spaces/newlines between the content and the $
    let mut math_block = false;
    let mut just_entered_math_block = false;
    let mut possible_non_inline = false;
    let mut last_space = false;
    let mut escaped = false;
    let mut math_buffer = String::new();
    for c in text.chars() {
        match c {
            '\\' if !escaped => escaped = true,
            '$' if !escaped => {
                math_buffer.push('$');
                if math_block {
                    let is_inline = !possible_non_inline || !last_space;
                    let math = render_math(std::mem::take(&mut math_buffer), is_inline);
                    *output += &math;
                } else {
                    just_entered_math_block = true;
                }
                math_block = !math_block;
            }
            c => {
                if c == ' ' {
                    last_space = true;
                    if just_entered_math_block {
                        possible_non_inline = true
                    }
                }

                just_entered_math_block = false;
                escaped = false;

                if math_block {
                    math_buffer.push(c);
                } else {
                    output.push(c)
                }
            }
        }
    }
}

fn start_tag(tagstack: &mut Vec<OurTag>, tag: Tag, output: &mut String) {
    match tag {
        Tag::Heading {
            level: HeadingLevel::H1,
            ..
        } => {
            tagstack.push(OurTag::H1);
            *output += r#"<h1 style="font-weight: 400;">"#;
        }
        Tag::Paragraph => {
            tagstack.push(OurTag::P);
            *output += r#"<p>"#;
        }
        Tag::CodeBlock(CodeBlockKind::Fenced(lang)) => {
            tagstack.push(OurTag::Code);
            *output += &format!("<pre><code class=\"lang-{lang}\">");
        }
        Tag::Image { dest_url, .. } => {
            *output += &format!("<img src=\"{}\">", dest_url.into_string());
        }
        _ => {}
    }
}

fn end_tag(tagstack: &mut Vec<OurTag>, tag: TagEnd, output: &mut String) {
    match tag {
        TagEnd::Heading(HeadingLevel::H1) => {
            assert_eq!(tagstack.pop(), Some(OurTag::H1));
            *output += r#"</h1>"#;
        }
        TagEnd::Paragraph => {
            assert_eq!(tagstack.pop(), Some(OurTag::P));
            *output += r#"</p>"#;
        }
        TagEnd::CodeBlock => {
            let tag = tagstack.pop();
            match tag {
                Some(OurTag::Code) => {
                    *output += r#"</code></pre>"#;
                }
                Some(OurTag::CodeInline) => {
                    *output += r#"</code>"#;
                }
                _ => unreachable!("got tag: {tag:?}"),
            }
        }
        _ => {}
    }
}

fn main() {
    env_logger::init();
    let articles = std::fs::read_dir("articles/").unwrap();
    for article in articles {
        let Ok(article) = article else { continue };
        std::fs::write(
            format!(
                "output/{}.html",
                article.path().file_stem().unwrap().to_string_lossy()
            ),
            parse_article(&article.path().to_string_lossy()),
        )
        .unwrap();
    }
}
