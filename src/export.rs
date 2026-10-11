//! Exporting the conversation as [CMF](https://sectorflabs.com/cmf/), the
//! Conversational Markdown Format: user messages are blockquotes, everything
//! else is the assistant's (ADR-0004).

use crate::app::ChatMessage;
use crate::openai::Role;

/// The conversation as CMF. System messages (the prompt, errors) have no
/// place in CMF and are left out. Replies before the first user message
/// become a preamble, which CMF readers skip but people can still read.
pub fn to_cmf(messages: &[ChatMessage]) -> String {
    let mut preamble = Vec::new();
    let mut turns: Vec<cmf::Turn> = Vec::new();
    for message in messages {
        match message.role {
            Role::System => {}
            Role::User => turns.push(cmf::Turn {
                user: cmf::UserMessage { username: None, content: message.content.trim().to_string() },
                assistant: String::new(),
            }),
            Role::Assistant => {
                let reply = escape(message.content.trim());
                match turns.last_mut() {
                    Some(turn) if turn.assistant.is_empty() => turn.assistant = reply,
                    Some(turn) => turn.assistant = format!("{}\n\n{reply}", turn.assistant),
                    None => preamble.push(reply),
                }
            }
        }
    }
    let body = cmf::Document { turns }.to_cmf();
    let mut out = preamble.join("\n\n");
    if !out.is_empty() && !body.is_empty() {
        out.push_str("\n\n");
    }
    out.push_str(&body);
    out.push('\n');
    out
}

/// Whether there's anything worth exporting.
pub fn has_content(messages: &[ChatMessage]) -> bool {
    messages.iter().any(|m| !matches!(m.role, Role::System))
}

/// A file name from the first thing the user asked, e.g. `whats-a-monad.cmf`.
pub fn file_name(messages: &[ChatMessage]) -> String {
    let first = messages.iter().find(|m| matches!(m.role, Role::User)).map(|m| m.content.as_str());
    let words: Vec<String> = first
        .unwrap_or_default()
        .split_whitespace()
        .map(|w| w.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase())
        .filter(|w| !w.is_empty())
        .take(6)
        .collect();
    if words.is_empty() { "chat.cmf".into() } else { format!("{}.cmf", words.join("-")) }
}

/// A reply line starting with `>` would read as the user's; CMF escapes it
/// by indenting one space.
fn escape(reply: &str) -> String {
    reply.lines().map(|l| if l.starts_with('>') { format!(" {l}") } else { l.to_string() }).collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(role: Role, content: &str) -> ChatMessage {
        ChatMessage { role, content: content.into() }
    }

    #[test]
    fn turns_round_trip_through_the_reference_parser() {
        let messages = [
            msg(Role::System, "You are a helpful assistant."),
            msg(Role::User, "What is 2+2?"),
            msg(Role::Assistant, "The answer is 4."),
            msg(Role::User, "Thanks!\nReally."),
            msg(Role::Assistant, "You're welcome.\n\n```rust\nfn main() {}\n```"),
        ];
        let text = to_cmf(&messages);
        assert!(text.starts_with("> What is 2+2?\nThe answer is 4.\n"), "{text}");
        assert!(text.contains("\n> Thanks!\n> Really.\nYou're welcome."), "{text}");
        let doc = cmf::Document::parse(&text);
        assert_eq!(doc.turns.len(), 2);
        assert_eq!(doc.turns[1].user.content, "Thanks!\nReally.");
        assert_eq!(doc.turns[1].assistant, "You're welcome.\n\n```rust\nfn main() {}\n```");
    }

    #[test]
    fn quotes_in_replies_stay_the_assistants() {
        let text = to_cmf(&[msg(Role::User, "Quote something"), msg(Role::Assistant, "Here:\n> to be or not to be")]);
        assert!(text.contains("\n > to be"), "{text}");
        assert_eq!(cmf::Document::parse(&text).turns.len(), 1);
    }

    #[test]
    fn errors_are_left_out_and_a_lone_question_still_exports() {
        let text = to_cmf(&[msg(Role::User, "Hello?"), msg(Role::System, "Error: connection refused")]);
        assert_eq!(text, "> Hello?\n");
    }

    #[test]
    fn the_file_is_named_after_the_first_question() {
        let messages = [msg(Role::System, "x"), msg(Role::User, "What's a monad, really?")];
        assert_eq!(file_name(&messages), "whats-a-monad-really.cmf");
        assert_eq!(file_name(&[]), "chat.cmf");
    }
}
