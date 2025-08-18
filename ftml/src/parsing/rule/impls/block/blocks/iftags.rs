/*
 * parsing/rule/impls/block/blocks/iftags.rs
 *
 * ftml - Library to parse Wikidot text
 * Copyright (C) 2019-2022 Wikijump Team
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

use super::prelude::*;
use crate::data::PageInfo;
use crate::parsing::ElementCondition;
use crate::parsing::parser::ParserTransactionFlags;

pub const BLOCK_IFTAGS: BlockRule = BlockRule {
    name: "block-iftags",
    accepts_names: &["iftags"],
    accepts_star: false,
    accepts_score: false,
    accepts_newlines: true,
    accepts_partial: AcceptsPartial::None,
    parse_fn,
};

fn parse_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
    name: &'t str,
    flag_star: bool,
    flag_score: bool,
    in_head: bool,
) -> ParseResult<'r, 't, Elements<'t>> {
    info!("Parsing iftags block (name '{name}', in-head {in_head})");
    assert!(!flag_star, "IfTags doesn't allow star flag");
    assert!(!flag_score, "IfTags doesn't allow score flag");
    assert_block_name(&BLOCK_IFTAGS, name);

    let no_conditionals = parser.settings().no_conditionals;
    let mut parser_tx = parser.transaction(ParserTransactionFlags::all());

    // Parse out tag conditions
    let conditions =
        parser_tx.get_head_value(&BLOCK_IFTAGS, in_head, |parser, spec| match spec {
            Some(spec) => Ok(ElementCondition::parse(spec.as_ref())),
            None => Err(parser.make_warn(ParseWarningKind::BlockMissingArguments)),
        })?;

    // Get body content, never with paragraphs
    let (elements, mut exceptions, paragraph_safe) =
        parser_tx.get_body_elements(&BLOCK_IFTAGS, name, false)?.into();

    debug!(
        "IfTags conditions parsed (conditions length {}, elements length {})",
        conditions.len(),
        elements.len(),
    );

    // Return elements based on condition
    let elements = if no_conditionals || check_iftags(parser_tx.page_info(), &conditions) {
        debug!("Conditions passed, including elements");

        // Confirm parser state modification caused by iftags content.
        parser_tx.commit();

        Elements::Multiple(elements)
    } else {
        debug!("Conditions failed, excluding elements");

        // Filter out non-warning exceptions
        exceptions.retain(|ex| matches!(ex, ParseException::Warning(_)));

        // Cancel all state modifications (forget TOC and footnotes).
        parser_tx.rollback();

        Elements::None
    };

    ok!(paragraph_safe; elements, exceptions)
}

pub fn check_iftags(info: &PageInfo, conditions: &[ElementCondition]) -> bool {
    debug!("Checking iftags");
    ElementCondition::check(conditions, &info.tags)
}
