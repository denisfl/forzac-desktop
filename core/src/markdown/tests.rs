//! The scenarios of `markdown-parsing`: the Android app's `MarkdownReaderTest`, ported, so that both apps read a
//! Markdown file the same way; and what only the desktop has (file names).

use super::*;

fn read_md(text: &str) -> Book {
    read(text)
}

fn blocks(book: &Book) -> Vec<&Block> {
    book.chapters.iter().flat_map(|c| &c.blocks).collect()
}

fn paras(book: &Book) -> Vec<(&StyledText, Kind)> {
    blocks(book).into_iter().filter_map(|b| if let Block::Paragraph { text, kind } = b { Some((text, *kind)) } else { None }).collect()
}

/// A paragraph's text, no-break spaces as spaces (the typography puts them in).
fn plain(book: &Book, i: usize) -> String {
    paras(book)[i].0.text.replace('\u{a0}', " ")
}

fn styled(text: &StyledText, style: Style) -> Vec<&str> {
    text.styles.iter().filter(|s| s.style == style).map(|s| &text.text[s.start..s.end]).collect()
}

#[test]
fn long_answer_with_sections() {
    let book = read_md("# Тема\n\n## Введение\n\nТекст.\n\n## Детали\n\n### Пример\n\nЕщё текст.\n\n## Итог\n\nКонец.\n");
    assert_eq!(book.title, "Тема");
    let titles: Vec<Option<&str>> = book.chapters.iter().map(|c| c.title.as_deref()).collect();
    assert_eq!(titles, [Some("Введение"), Some("Детали"), Some("Итог")]);
    let toc: Vec<(&str, u32)> = book.toc.iter().map(|e| (e.title.as_str(), e.level)).collect();
    assert_eq!(toc, [("Введение", 1), ("Детали", 1), ("Пример", 2), ("Итог", 1)]);
    assert_eq!(book.toc[2].target, Locator { chapter: 1, block: 1, offset: 0 });
    assert!(matches!(book.chapters[1].blocks[1], Block::Heading { level: 2, .. }));
}

#[test]
fn several_top_headings_are_chapters() {
    let book = read_md("Вступление.\n\n# Один\n\nА.\n\n# Два\n\nБ.");
    assert_eq!(book.title, "");
    let titles: Vec<Option<&str>> = book.chapters.iter().map(|c| c.title.as_deref()).collect();
    assert_eq!(titles, [None, Some("Один"), Some("Два")]);
}

#[test]
fn front_matter() {
    let book = read_md("---\ntitle: Заметки\nauthor: Денис\nlang: ru\n---\nТекст.\n");
    assert_eq!(book.title, "Заметки");
    assert_eq!(book.authors, ["Денис"]);
    assert_eq!(book.language.as_deref(), Some("ru"));
    let texts: Vec<&str> = paras(&book).iter().map(|p| p.0.text.as_str()).collect();
    assert_eq!(texts, ["Текст."]);
}

#[test]
fn setext_headings() {
    let book = read_md("Первая\n======\n\nА.\n\nВторая\n------\n\nБ.");
    assert_eq!(book.title, "Первая");
    let toc: Vec<&str> = book.toc.iter().map(|e| e.title.as_str()).collect();
    assert_eq!(toc, ["Вторая"]);
}

#[test]
fn paragraphs_and_hard_breaks() {
    let book = read_md("Строка один\nи её продолжение.  \nПосле жёсткого переноса\\\nи ещё.");
    assert_eq!(plain(&book, 0), "Строка один и её продолжение.\nПосле жёсткого переноса\nи ещё.");
}

#[test]
fn quotes_and_nested_quotes() {
    let book = read_md("> Цитата\n> продолжается.\n>\n> > Вложенная.");
    let cites = paras(&book);
    assert_eq!(cites.iter().map(|p| p.0.text.as_str()).collect::<Vec<_>>(), ["Цитата продолжается.", "Вложенная."]);
    assert!(cites.iter().all(|p| p.1 == Kind::Cite));
}

#[test]
fn fenced_code_keeps_spaces() {
    let book = read_md("Пример:\n\n```kotlin\nfun main() {\n    println(\"*not em*\")\n}\n```\n");
    let (code, kind) = paras(&book)[1];
    assert_eq!(kind, Kind::Code);
    assert_eq!(code.text, "fun main() {\n    println(\"*not em*\")\n}");
}

#[test]
fn indented_code() {
    let book = read_md("Текст.\n\n    a = 1\n    b = 2\n\nДальше.");
    assert_eq!(paras(&book)[1].0.text, "a = 1\nb = 2");
    assert_eq!(paras(&book)[1].1, Kind::Code);
}

#[test]
fn lists() {
    let book = read_md("- один\n- два\n  продолжение\n\n1. первый\n2. второй\n\n- [ ] сделать\n- [x] сделано");
    let texts: Vec<&str> = paras(&book).iter().map(|p| p.0.text.as_str()).collect();
    assert_eq!(texts, ["• один", "• два продолжение", "1. первый", "2. второй", "☐ сделать", "☑ сделано"]);
    assert!(paras(&book).iter().all(|p| matches!(p.1, Kind::ListItem(_))));
}

#[test]
fn nested_list_levels() {
    let book = read_md("- один\n  - вложенный\n    - глубже\n  - снова второй\n- верхний");
    let levels: Vec<u32> = paras(&book).iter().map(|p| if let Kind::ListItem(level) = p.1 { level } else { 99 }).collect();
    assert_eq!(levels, [0, 1, 2, 1, 0]);
}

#[test]
fn indented_paragraph_inside_list_is_not_code() {
    let book = read_md("1. Шаг\n\n    Пояснение к шагу.\n\nДальше.");
    assert_eq!(paras(&book)[1].1, Kind::Body);
    assert_eq!(plain(&book, 1), "Пояснение к шагу.");
}

#[test]
fn rule_is_scene_break() {
    let book = read_md("А.\n\n---\n\nБ.");
    assert_eq!(blocks(&book)[1], &Block::EmptyLine);
}

#[test]
fn table() {
    let book = read_md("| Имя | Цена |\n|:----|-----:|\n| **Чай** | 3 |\n| Кофе \\| латте | 5 |");
    let rows: Vec<&Vec<TableCell>> = blocks(&book)
        .into_iter()
        .filter_map(|b| if let Block::TableRow { table, row } = b { Some(&table.rows[*row]) } else { None })
        .collect();
    assert_eq!(rows.len(), 3);
    assert!(rows[0].iter().all(|c| c.header));
    assert_eq!(rows[1].iter().map(|c| c.text.text.as_str()).collect::<Vec<_>>(), ["Чай", "3"]);
    assert_eq!(styled(&rows[1][0].text, Style::Strong), ["Чай"]);
    assert_eq!(rows[2][0].text.text, "Кофе | латте");
}

#[test]
fn inline_styles() {
    let book = read_md("*курсив*, **жирный**, ***оба***, ~~зачёркнутый~~, `код *не курсив*`, snake_case_name.");
    let text = paras(&book)[0].0;
    assert_eq!(text.text, "курсив, жирный, оба, зачёркнутый, код *не курсив*, snake_case_name.");
    assert_eq!(styled(text, Style::Emphasis), ["курсив", "оба"]);
    assert_eq!(styled(text, Style::Strong), ["жирный", "оба"]);
    assert_eq!(styled(text, Style::Strikethrough), ["зачёркнутый"]);
    assert_eq!(styled(text, Style::Code), ["код *не курсив*"]);
}

#[test]
fn escaped_star() {
    assert_eq!(paras(&read_md("2 \\* 3 = 6"))[0].0.text, "2 * 3 = 6");
}

#[test]
fn unmatched_delimiters_stay_as_text() {
    assert_eq!(plain(&read_md("цена *5 и **скидка"), 0), "цена *5 и **скидка");
}

#[test]
fn external_links() {
    let book = read_md("Сайт [Форзаца](https://forzac.truenrth.app), <mailto:denis@fedosov.me> и https://example.com/a_(b).");
    let text = paras(&book)[0].0;
    let links: Vec<&StyleRange> = text.styles.iter().filter(|s| s.style == Style::Link).collect();
    let shown: Vec<&str> = links.iter().map(|s| &text.text[s.start..s.end]).collect();
    assert_eq!(shown, ["Форзаца", "mailto:denis@fedosov.me", "https://example.com/a_(b)"]);
    assert_eq!(book.links.get(links[0].target.as_deref().unwrap()), Some(&LinkTarget::External("https://forzac.truenrth.app".into())));
    assert!(text.text.ends_with("(b)."));
}

#[test]
fn anchor_links_lead_to_headings() {
    let book = read_md("# Тема\n\n- [Детали](#детали)\n- [Нет такого](#нет)\n\n## Введение\n\nА.\n\n## Детали\n\nБ.");
    let first = &book.chapters[0].blocks;
    let Block::Paragraph { text, .. } = &first[0] else { panic!() };
    let link: Vec<&StyleRange> = text.styles.iter().filter(|s| s.style == Style::Link).collect();
    assert_eq!(link.len(), 1);
    assert_eq!(
        book.links.get(link[0].target.as_deref().unwrap()),
        Some(&LinkTarget::Place { start: Locator { chapter: 2, block: 0, offset: 0 }, end: None })
    );
    let Block::Paragraph { text, .. } = &first[1] else { panic!() };
    assert!(text.styles.iter().all(|s| s.style != Style::Link));
}

#[test]
fn relative_links_are_text() {
    let book = read_md("См. [файл](notes/other.md).");
    let text = paras(&book)[0].0;
    assert_eq!(text.text, "См. файл.");
    assert!(text.styles.is_empty());
}

#[test]
fn images_become_italic_captions() {
    let book = read_md("![Схема движка](img/engine.png)\n\n![](x.png)\n\nТекст.");
    let first = paras(&book)[0].0;
    assert_eq!(first.text, "Схема движка");
    assert_eq!(first.styles, [StyleRange { start: 0, end: "Схема движка".len(), style: Style::Emphasis, target: None }]);
    assert_eq!(paras(&book)[1].0.text, "Текст.");
}

#[test]
fn html_tags_are_stripped() {
    assert_eq!(plain(&read_md("Один<br>два <b>и</b> три.<!-- заметка -->"), 0), "Один\nдва и три.");
}

#[test]
fn typography_outside_code() {
    assert_eq!(paras(&read_md("Он сказал - да. Код: `x - y`."))[0].0.text, "Он сказал\u{a0}— да. Код: x - y.");
}

#[test]
fn markdown_in_a_txt_file_and_a_title_from_the_name() {
    let dir = std::env::temp_dir().join(format!("forzac-markdown-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let answer = dir.join("answer.txt");
    std::fs::write(&answer, "# Сравнение\n\n| Формат | Год |\n|---|---|\n| FB2 | 2004 |\n").unwrap();
    let book = crate::open(&answer).unwrap();
    assert_eq!(book.title, "Сравнение", "Markdown by its content, whatever the name");
    assert!(matches!(book.chapters[0].blocks[0], Block::TableRow { .. }));
    let untitled = dir.join("ответ.md");
    std::fs::write(&untitled, "## Коротко\n\nТекст.\n").unwrap();
    assert_eq!(crate::open(&untitled).unwrap().title, "ответ", "no first-level heading: the file's name");
    std::fs::remove_dir_all(&dir).unwrap();
}
