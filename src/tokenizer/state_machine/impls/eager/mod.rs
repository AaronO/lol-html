#[macro_use]
mod actions;
mod conditions;

use crate::base::{Align, Chunk, Cursor, Range};

use crate::tokenizer::outputs::*;
use crate::tokenizer::state_machine::{
    ParsingLoopDirective, ParsingLoopTerminationReason, StateMachine, StateResult,
};
use crate::tokenizer::{
    FeedbackProviders, NextOutputType, TagName, TagPreviewHandler, TextParsingMode,
    TreeBuilderFeedback,
};
use crate::Error;
use std::cell::RefCell;
use std::cmp::min;
use std::rc::Rc;

#[cfg(feature = "testing_api")]
use super::common::SharedTagConfirmationHandler;

pub type State<H> = fn(&mut EagerStateMachine<H>, &Chunk<'_>) -> StateResult;

/// Eager state machine skips the majority of full state machine operations and, thus,
/// is faster. It also has much less requirements for buffering which makes it more
/// prone to bailouts caused by buffer exhaustion (actually it buffers only tag names).
///
/// Eager state machine produces tag previews as an output which serve as a hint for
/// the matcher which can then switch to the full state machine if required.
///
/// It's not guaranteed that tag preview will actually produce the token in the end
/// of the input (e.g. `<div` will produce a tag preview, but not tag token). However,
/// it's not a concern for our use case as no content will be erroneously captured
/// in this case.
pub struct EagerStateMachine<H: TagPreviewHandler> {
    input_cursor: Cursor,
    tag_start: Option<usize>,
    ch_sequence_matching_start: Option<usize>,
    tag_name_start: usize,
    is_in_end_tag: bool,
    tag_name_hash: Option<u64>,
    last_start_tag_name_hash: Option<u64>,
    state_enter: bool,
    allow_cdata: bool,
    tag_preview_handler: H,
    state: State<H>,
    closing_quote: u8,
    feedback_providers: Rc<RefCell<FeedbackProviders>>,
    pending_text_parsing_mode_change: Option<TextParsingMode>,
    last_text_parsing_mode_change: TextParsingMode,

    #[cfg(feature = "testing_api")]
    pub tag_confirmation_handler: Option<SharedTagConfirmationHandler>,
}

impl<H: TagPreviewHandler> EagerStateMachine<H> {
    pub fn new(tag_preview_handler: H, feedback_providers: Rc<RefCell<FeedbackProviders>>) -> Self {
        EagerStateMachine {
            input_cursor: Cursor::default(),
            tag_start: None,
            ch_sequence_matching_start: None,
            tag_name_start: 0,
            is_in_end_tag: false,
            tag_name_hash: None,
            last_start_tag_name_hash: None,
            state_enter: true,
            allow_cdata: false,
            tag_preview_handler,
            state: EagerStateMachine::data_state,
            closing_quote: b'"',
            feedback_providers,
            pending_text_parsing_mode_change: None,
            last_text_parsing_mode_change: TextParsingMode::Data,

            #[cfg(feature = "testing_api")]
            tag_confirmation_handler: None,
        }
    }

    fn create_tag_preview<'i>(&mut self, input: &'i Chunk<'i>) -> TagPreview<'i> {
        let name_range = Range {
            start: self.tag_name_start,
            end: self.input_cursor.pos(),
        };

        let tag_name_info = TagNameInfo::new(input, name_range, self.tag_name_hash);

        if self.is_in_end_tag {
            self.is_in_end_tag = false;
            TagPreview::EndTag(tag_name_info)
        } else {
            self.last_start_tag_name_hash = self.tag_name_hash;
            TagPreview::StartTag(tag_name_info)
        }
    }

    fn get_feedback_for_tag(
        &mut self,
        tag_preview: &TagPreview<'_>,
    ) -> Result<TreeBuilderFeedback, Error> {
        let mut feedback_providers = self.feedback_providers.borrow_mut();

        match *tag_preview {
            TagPreview::StartTag(TagNameInfo { name_hash, .. }) => {
                feedback_providers
                    .ambiguity_guard
                    .track_start_tag(name_hash)?;

                Ok(feedback_providers
                    .tree_builder_simulator
                    .get_feedback_for_start_tag_name(name_hash))
            }
            TagPreview::EndTag(TagNameInfo { name_hash, .. }) => {
                feedback_providers.ambiguity_guard.track_end_tag(name_hash);

                Ok(feedback_providers
                    .tree_builder_simulator
                    .get_feedback_for_end_tag_name(name_hash))
            }
        }
    }

    fn handle_tree_builder_feedback(
        &mut self,
        feedback: TreeBuilderFeedback,
        tag_start: usize,
    ) -> ParsingLoopDirective {
        match feedback {
            TreeBuilderFeedback::SwitchTextParsingMode(mode) => {
                // NOTE: we can't switch mode immediately as we
                // are in the middle of tag parsing. So, we need
                // to switch later on the `emit_tag` action.
                self.pending_text_parsing_mode_change = Some(mode);
                ParsingLoopDirective::None
            }
            TreeBuilderFeedback::SetAllowCdata(allow_cdata) => {
                self.allow_cdata = allow_cdata;
                ParsingLoopDirective::None
            }
            TreeBuilderFeedback::RequestLexUnit(_) => ParsingLoopDirective::Break(
                ParsingLoopTerminationReason::LexUnitRequiredForAdjustment(
                    self.create_bookmark(tag_start),
                ),
            ),
            TreeBuilderFeedback::None => ParsingLoopDirective::None,
        }
    }
}

impl<H: TagPreviewHandler> StateMachine for EagerStateMachine<H> {
    impl_common_sm_accessors!();

    #[inline]
    fn set_state(&mut self, state: State<H>) {
        self.state = state;
    }

    #[inline]
    fn get_state(&self) -> State<H> {
        self.state
    }

    #[inline]
    fn get_blocked_byte_count(&self, input: &Chunk<'_>) -> usize {
        // NOTE: if we are in character sequence matching we need
        // to block from the position where matching starts. We don't
        // need to do that manually in full state machine because it
        // always blocks all bytes starting from lex unit start and it's
        // guaranteed that character sequence matching occurs withih
        // lex unit boundaries.
        match (self.tag_start, self.ch_sequence_matching_start) {
            (Some(tag_start), Some(ch_sequence_matching_start)) => {
                input.len() - min(tag_start, ch_sequence_matching_start)
            }
            (Some(tag_start), None) => input.len() - tag_start,
            (None, Some(ch_sequence_matching_start)) => input.len() - ch_sequence_matching_start,
            (None, None) => 0,
        }
    }

    fn adjust_for_next_input(&mut self) {
        if let Some(tag_start) = self.tag_start {
            self.input_cursor.align(tag_start);
            self.tag_name_start.align(tag_start);
            self.tag_start = Some(0);
        } else {
            self.input_cursor = Cursor::default();
        }
    }

    #[inline]
    fn adjust_to_bookmark(&mut self, _pos: usize) {
        trace!(@noop);
    }

    #[inline]
    fn store_last_text_parsing_mode_change(&mut self, mode: TextParsingMode) {
        self.last_text_parsing_mode_change = mode;
    }

    #[inline]
    fn enter_ch_sequence_matching(&mut self) {
        self.ch_sequence_matching_start = Some(self.input_cursor.pos());
    }

    #[inline]
    fn leave_ch_sequence_matching(&mut self) {
        self.ch_sequence_matching_start = None;
    }
}