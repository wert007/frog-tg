use lopdf::{
    Document, FontData, Object, Stream,
    content::{Content, Operation},
    dictionary,
};

use crate::CompleteWalk;

const FONT: &[u8] = include_bytes!("../../assets/fonts/Coolvetica Rg.otf");
const TEMPLATE: &[u8] = include_bytes!("../../assets/template.pdf");

pub fn create_pdf_report(walk: &CompleteWalk) -> anyhow::Result<()> {
    let mut doc = lopdf::Document::load_mem(TEMPLATE)?;
    doc.add_font(FontData::new(FONT, "default".into()))?;

    let pages = doc.get_pages();
    let page_id = *pages.get(&1).unwrap();

    write_date(&mut doc, page_id, walk.start)?;
    // write(&mut doc, "test", 12, [10, 10], page_id)?;

    doc.save("output.pdf")?;

    Ok(())
}

fn write_date(
    doc: &mut Document,
    page_id: (u32, u16),
    start: chrono::DateTime<chrono::Local>,
) -> anyhow::Result<()> {
    write(
        doc,
        start.format("%d.%m.%Y").to_string(),
        12,
        [100, 130],
        page_id,
    )
}

fn write(
    doc: &mut Document,
    text: impl AsRef<str>,
    size: i32,
    position: [i32; 2],
    page_id: (u32, u16),
) -> anyhow::Result<()> {
    let content = Content {
        operations: vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["default".into(), size.into()]), // font + size
            Operation::new(
                "Tm",
                vec![
                    1.into(),
                    0.into(),
                    0.into(),
                    (-1).into(),
                    position[0].into(),
                    position[1].into(),
                ],
            ),
            // Operation::new("Td", vec![10.into(), 10.into()]),        // x,y position
            Operation::new("Tj", vec![Object::string_literal(text.as_ref())]),
            Operation::new("ET", vec![]),
        ],
    };

    let stream = Stream::new(dictionary! {}, content.encode()?);
    let content_id = doc.add_object(stream);

    {
        let page = doc.get_object_mut(page_id)?.as_dict_mut()?;

        match page.get_mut(b"Contents") {
            Ok(Object::Reference(existing)) => {
                let existing = *existing;
                page.set(
                    "Contents",
                    Object::Array(vec![
                        Object::Reference(existing),
                        Object::Reference(content_id),
                    ]),
                );
            }
            Ok(Object::Array(arr)) => {
                arr.push(Object::Reference(content_id));
            }
            _ => {
                page.set("Contents", content_id);
            }
        }
    }
    Ok(())
}
