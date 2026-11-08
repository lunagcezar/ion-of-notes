//! Neutral demo workspace used on first start.

use super::{Block, BlockKind, Page, TableRow, Workspace};

impl Workspace {
    /// A small, neutral demo workspace used on first start.
    pub fn seed(name: &str) -> Self {
        fn make(id: &str, title: &str, icon: &str, parent: Option<&str>) -> Page {
            let mut p = Page::new(id, title, icon);
            p.parent = parent.map(str::to_string);
            p
        }

        let mut ws = Workspace::new(name);

        // Neutral demo content shaped like the pictured document page: a
        // callout, bold section headings, body paragraphs and a short list.
        // An empty icon is the default open-book artwork. The other pages use
        // customizable glyphs so the choice is also visible in the sidebar,
        // breadcrumbs and tabs (spec-013).
        let mut home = make("p-home", "Home", "", None);
        home.favorite = true;
        home.blocks = vec![
            Block::new(
                "h1",
                BlockKind::Callout,
                "A shared workspace for notes, projects and tasks. Sign in to your self-hosted server: changes sync across devices, with sharing and live editing for the whole team.",
            ),
            Block::new("h2", BlockKind::Heading1, "Overview"),
            Block::new(
                "h3",
                BlockKind::Paragraph,
                "Pages are grouped into a simple tree. Use the sidebar to move between them, and click any line to edit it in place.",
            ),
            Block::new(
                "h4",
                BlockKind::Paragraph,
                "Press Ctrl+K for quick search, or type / inside a block to change its type. Changes are saved automatically as you work.",
            ),
            Block::new("h5", BlockKind::Heading1, "Documentation"),
            Block::new("h6", BlockKind::Bulleted, "01 · About this project"),
            Block::new("h7", BlockKind::Bulleted, "02 · Search and references"),
        ];
        ws.add_page(home).expect("seed: home");

        let mut start = make("p-start", "Getting Started", "✦", None);
        start.blocks = vec![
            Block::new("s1", BlockKind::Heading1, "Getting Started"),
            Block::new(
                "s2",
                BlockKind::Paragraph,
                "This is a short guide to the workspace. Edit any line by clicking on it.",
            ),
            Block::new("s3", BlockKind::Heading2, "Things to try"),
            Block::new("s4", BlockKind::Todo, "Rename this page"),
            Block::new("s5", BlockKind::Todo, "Create a child page"),
            Block::new("s6", BlockKind::Todo, "Open the Tasks table"),
            Block::new("s7", BlockKind::Quote, "Small steps, written down, add up."),
        ];
        ws.add_page(start).expect("seed: getting started");

        let mut projects = make("p-projects", "Projects", "◆", None);
        projects.blocks = vec![
            Block::new("p1", BlockKind::Heading1, "Projects"),
            Block::new(
                "p2",
                BlockKind::Paragraph,
                "Group related pages together. A page can be nested under another page.",
            ),
            Block::new("p3", BlockKind::Heading3, "Active work"),
            Block::new("p4", BlockKind::Bulleted, "Tasks"),
            Block::new("p5", BlockKind::Bulleted, "Notes"),
        ];
        ws.add_page(projects).expect("seed: projects");

        let mut tasks = make("p-tasks", "Tasks", "☑", Some("p-projects"));
        tasks.favorite = true;
        tasks.blocks = vec![
            Block::new("t1", BlockKind::Heading1, "Tasks"),
            Block::new(
                "t2",
                BlockKind::Paragraph,
                "A simple table to track work. Switch between the Table and Board views.",
            ),
        ];
        tasks.rows = vec![
            TableRow::new(
                "r1",
                "Draft the project brief",
                "In progress",
                "Ava",
                "Nov 06",
            ),
            TableRow::new("r2", "Review the design spec", "Done", "Luna", "Nov 04"),
            TableRow::new(
                "r3",
                "Set up the data model",
                "Not started",
                "Ava",
                "Nov 10",
            ),
            TableRow::new(
                "r4",
                "Write the first spec",
                "In progress",
                "Luna",
                "Nov 07",
            ),
            TableRow::new("r5", "Polish the sidebar", "Not started", "Milo", "Nov 12"),
        ];
        ws.add_page(tasks).expect("seed: tasks");

        let mut notes = make("p-notes", "Notes", "✎", None);
        notes.blocks = vec![
            Block::new("n1", BlockKind::Heading1, "Notes"),
            Block::new(
                "n2",
                BlockKind::Paragraph,
                "A scratch pad for ideas that do not belong anywhere yet.",
            ),
            Block::new("n3", BlockKind::Code, "let idea = \"write it down\";"),
            Block::new("n4", BlockKind::Divider, ""),
            Block::new(
                "n5",
                BlockKind::Paragraph,
                "Add a new block with the plus button on the left.",
            ),
        ];
        ws.add_page(notes).expect("seed: notes");

        ws
    }
}
