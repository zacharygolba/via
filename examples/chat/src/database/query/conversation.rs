use diesel::prelude::*;
use diesel::result::Error as DieselError;
use fred::types::Key;
use via::request::PathParams;
use via::{Error, raise};

use crate::database::Connection;
use crate::models::conversation::*;
use crate::models::reaction::{Reaction, ReactionPreview};
use crate::util::paginate::PER_PAGE;
use crate::util::{DebugQueryDsl, Id};

#[derive(Clone)]
pub struct ConversationParams {
    pub thread_id: Id,
    pub reply_id: Option<Id>,
}

pub async fn conversation_by_id(
    connection: &mut Connection<'_>,
    params: &ConversationParams,
) -> via::Result<ThreadDetails> {
    let id = params.id();

    // Load the conversation by id along with the first page of replies.
    let mut conversations = Conversation::with_author()
        .select(ConversationWithUser::as_select())
        .filter(by_id(id).or(by_thread(id)))
        .order(recent())
        .limit(PER_PAGE + 1)
        .debug_load(connection)
        .await?;

    // Aggregate the reactions for each conversation.
    let mut reactions = {
        let ids = conversations.iter().map(Identifiable::id);
        Reaction::to_conversations(connection, ids).await?
    };

    // Take the last conversation from the vec of conversations. The query
    // orders conversations by created_at DESC. Therefore, the last item in the
    // vec is always the parent.
    let Some(conversation) = conversations.pop() else {
        raise!(404, DieselError::NotFound);
    };

    // Move the reactions to conversation into their own vec.
    let own_reactions = split_own_reactions(&mut reactions, id);

    // Group replies to the thread with with their reactions.
    let replies = ConversationDetails::grouped_by(conversations, reactions);

    Ok(conversation.into_thread(own_reactions, replies))
}

fn split_own_reactions(reactions: &mut Vec<ReactionPreview>, id: &Id) -> Vec<ReactionPreview> {
    // Sort the reactions vec so that our own reactions are last.
    reactions.sort_by_key(|reaction| *id == reaction.to_id());

    // Find the first index of a reaction that belongs to the id predicate.
    let pivot = reactions
        .iter()
        .position(|reaction| *id == reaction.to_id())
        .unwrap_or(reactions.len());

    reactions.split_off(pivot)
}

impl ConversationParams {
    pub fn id(&self) -> &Id {
        self.reply_id.as_ref().unwrap_or(&self.thread_id)
    }

    pub fn key(&self) -> String {
        format!("conversations:{}", self.id())
    }
}

impl From<&'_ ConversationParams> for Key {
    fn from(value: &'_ ConversationParams) -> Self {
        format!("conversations:{}", value.id()).into()
    }
}

impl TryFrom<PathParams<'_>> for ConversationParams {
    type Error = Error;

    fn try_from(params: PathParams<'_>) -> Result<Self, Self::Error> {
        Ok(Self {
            thread_id: params.get("thread-id").parse()?,
            reply_id: params
                .get("reply-id")
                .optional()?
                .map(|id| id.parse())
                .transpose()?,
        })
    }
}
