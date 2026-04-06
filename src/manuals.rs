use anyhow::{Context, Result, bail};
use firefly_types::manuals::*;
use markdown::mdast;

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
            if !node.ordered {
                bail!("unordered list is not supported yet");
            }
        }
        mdast::Node::ListItem(node) => {
            let nodes = parse_paragraph(&node.children).context("list item")?;
            let block = Block::Oli(nodes);
            page.content.push(block);
        }
        mdast::Node::Paragraph(node) => {
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
        let inline = parse_inline(node)?;
        paragraph.push(inline);
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
