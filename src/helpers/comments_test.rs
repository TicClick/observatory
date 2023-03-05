use super::*;

#[test]
fn to_markdown() {
    let hdr = CommentHeader {
        pull_number: 12,
        conflict_type: ConflictType::Overlap,
    };
    assert_eq!(
        hdr.to_markdown(),
        r#"<!--
pull_number: 12
conflict_type: Overlap
-->"#
    );
}

#[test]
fn from_comment_without_header() {
    let comment = "test comment";
    assert_eq!(CommentHeader::from_comment(comment), None);
}

#[test]
fn from_comment_with_bad_header() {
    let c1 = r#"<!--
test comment"#;
    assert_eq!(CommentHeader::from_comment(c1), None);

    let c2 = r#"<!--
pull_number: 12
some shit
conflict_type: Overlap
"#;
    assert_eq!(CommentHeader::from_comment(c2), None);
}

#[test]
fn from_comment_ok() {
    let comment = r#"<!--
pull_number: 12
conflict_type: Overlap
-->
Some text here."#;
    assert_eq!(
        CommentHeader::from_comment(comment),
        Some(CommentHeader {
            pull_number: 12,
            conflict_type: ConflictType::Overlap
        })
    );
}
