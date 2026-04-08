use std::path::Path;

use anyhow::{Context, Result, bail};
use firefly_types::{Encode, manuals::*};
use markdown::mdast;

pub fn convert_manual(in_path: &Path, out_path: &Path) -> Result<()> {
    let manual = parse_manual(in_path).context("parse")?;
    let raw = manual.encode_vec().context("serialize")?;
    std::fs::write(out_path, &raw).context("write")?;
    Ok(())
}

/// Read and parse manual from the given path.
fn parse_manual(path: &Path) -> Result<Manual> {
    let mut manual = Manual { pages: Vec::new() };

    // A single-page manual can be a single file.
    if path.is_file() {
        let content = std::fs::read_to_string(path).context("read manual file")?;
        let page = parse_page(&content).context("parse manual page")?;
        manual.pages.push(page);
        return Ok(manual);
    }

    let dir = std::fs::read_dir(path).context("open manual dir")?;
    let mut paths = Vec::new();
    for entry in dir {
        let entry = entry.context("access manual file")?;
        let path = entry.path();
        let Some(ext) = path.extension() else {
            bail!("the file name has no extension");
        };
        if ext != "md" {
            bail!("invalid file extension")
        }
        paths.push(path);
    }
    paths.sort();

    for path in paths {
        let content = std::fs::read_to_string(path).context("read manual file")?;
        let page = parse_page(&content).context("parse manual page")?;
        manual.pages.push(page);
    }
    Ok(manual)
}

fn parse_page(content: &str) -> Result<Page> {
    let options = markdown::ParseOptions::default();
    let ast = markdown::to_mdast(content, &options).unwrap();
    let mut page = Page {
        title: String::new(),
        badge: None,
        score: None,
        theme: None,
        content: Vec::new(),
    };
    let mdast::Node::Root(root) = ast else {
        bail!("invalid root node")
    };
    for node in root.children {
        parse_block(&mut page, node)?;
    }
    Ok(page)
}

fn parse_block(page: &mut Page, node: mdast::Node) -> Result<()> {
    match node {
        mdast::Node::Blockquote(node) => {
            let nodes = parse_paragraph(&node.children).context("blockquote")?;
            let block = Block::Quote(nodes);
            page.content.push(block);
        }
        mdast::Node::Heading(node) => {
            let depth = node.depth;
            let text = mdast::Node::Heading(node).to_string();
            if depth == 1 {
                page.title = text;
                return Ok(());
            }
            let block = if depth == 2 {
                Block::H2(text)
            } else {
                Block::H3(text)
            };
            page.content.push(block);
        }
        mdast::Node::List(node) => {
            for subnode in node.children {
                let mdast::Node::ListItem(item) = subnode else {
                    bail!("list contains unexpected node");
                };
                let nodes = parse_paragraph(&item.children).context("list item")?;
                let block = if node.ordered {
                    Block::Oli(nodes)
                } else {
                    Block::Uli(nodes)
                };
                page.content.push(block);
            }
        }
        mdast::Node::Paragraph(node) => {
            if node.children.len() == 1 {
                let subnode = &node.children[0];
                match subnode {
                    mdast::Node::Link(link) => {
                        if !link.url.starts_with("https://") {
                            bail!("only external links are supported yet");
                        }
                        let block = Block::Qr(link.url.clone());
                        page.content.push(block);
                        return Ok(());
                    }
                    mdast::Node::Image(_) => bail!("images are not supported yet"),
                    _ => {}
                }
            }
            let nodes = parse_paragraph(&node.children).context("paragraph")?;
            let block = Block::P(nodes);
            page.content.push(block);
        }
        _ => bail!("unsupported Markdown block node type"),
    }
    Ok(())
}

fn parse_paragraph(nodes: &[mdast::Node]) -> Result<Paragraph> {
    let mut paragraph = Paragraph::new();
    for node in nodes {
        if let mdast::Node::Paragraph(n) = node {
            for node in &n.children {
                let inline = parse_inline(node)?;
                paragraph.push(inline);
            }
        } else {
            let inline = parse_inline(node)?;
            paragraph.push(inline);
        }
    }
    Ok(paragraph)
}

fn parse_inline(node: &mdast::Node) -> Result<Inline> {
    let kind = match &node {
        mdast::Node::Emphasis(_) => InlineKind::Italic,
        // mdast::Node::Html(_) => InlineKind::Plain,
        // mdast::Node::Image(_) => InlineKind::Plain,
        // mdast::Node::Link(_) => InlineKind::Plain,
        mdast::Node::Strong(_) => InlineKind::Bold,
        mdast::Node::Text(_) => InlineKind::Plain,
        _ => bail!("unsupported Markdown inline node type"),
    };
    let content = node.to_string();
    Ok(Inline { kind, content })
}
