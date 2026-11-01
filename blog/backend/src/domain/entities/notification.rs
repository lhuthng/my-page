/// Notification kinds shared by the in-app writer, the email dispatcher, and
/// the API. `Reply` is the most specific reason, `PostComment` the broadest;
/// that order also decides which row survives deduplication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NotificationKind {
    Reply,
    Mention,
    PostComment,
}

impl NotificationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            NotificationKind::Reply => "reply",
            NotificationKind::Mention => "mention",
            NotificationKind::PostComment => "post_comment",
        }
    }

    /// Lower rank wins when one recipient qualifies for several kinds on the
    /// same comment.
    pub fn priority(self) -> u8 {
        match self {
            NotificationKind::Reply => 0,
            NotificationKind::Mention => 1,
            NotificationKind::PostComment => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommentRecipient {
    pub user_id: i64,
    pub kind: NotificationKind,
}

#[derive(Debug, Clone)]
pub struct CommentCreated {
    pub comment_id: i64,
    pub recipients: Vec<CommentRecipient>,
}
