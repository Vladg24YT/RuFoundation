/*
 * parsing/mod.rs
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

#[macro_use]
mod macros;

mod boolean;
mod check_step;
mod collect;
mod condition;
mod consume;
mod depth;
mod element_condition;
mod exception;
mod outcome;
mod paragraph;
mod parser;
mod result;
mod rule;
mod string;
mod strip;
mod token;

mod prelude {
    pub use crate::parsing::{
        ExtractedToken, ParseException, ParseResult, ParseSuccess, ParseWarning,
        ParseWarningKind, Token,
    };
    pub use crate::settings::WikitextSettings;
    pub use crate::text::FullText;
    pub use crate::tree::{Element, Elements};
}

use self::depth::{process_depths, DepthItem, DepthList};
use self::element_condition::{ElementCondition, ElementConditionType};
use self::paragraph::{gather_paragraphs, NO_CLOSE_CONDITION};
use self::parser::Parser;
use self::rule::impls::RULE_PAGE;
use self::string::parse_string;
use self::strip::{strip_newlines, strip_whitespace};
use crate::data::{PageCallbacks, PageInfo, PageRef};
use crate::next_index::{NextIndex, TableOfContentsIndex};
use crate::settings::WikitextSettings;
use crate::tokenizer::Tokenization;
use crate::tree::{
    AttributeMap, Element, LinkLabel, LinkLocation, LinkType,
    SyntaxTree, Container, ContainerType,
};
use std::borrow::Cow;
use std::collections::HashMap;
use std::rc::Rc;

pub use self::boolean::{parse_boolean, NonBooleanValue};
pub use self::exception::{ParseException, ParseWarning, ParseWarningKind};
pub use self::outcome::ParseOutcome;
pub use self::result::{ParseResult, ParseSuccess};
pub use self::token::{ExtractedToken, Token};

pub type WikiScriptScope<'t> = HashMap<Cow<'t, str>, (Cow<'t, str>, u32)>;

/// Parse through the given tokens and produce an AST.
///
/// This takes a list of `ExtractedToken` items produced by `tokenize()`.
pub fn parse<'r, 't>(
    tokenization: &'r Tokenization<'t>,
    page_info: &'r PageInfo<'t>,
    page_callbacks: Rc<dyn PageCallbacks>,
    settings: &'r WikitextSettings,
) -> ParseOutcome<SyntaxTree<'t>>
where
    'r: 't,
{
    // Run parsing, get raw results
    let UnstructuredParseResult {
        result,
        table_of_contents_depths,
        footnotes,
        code,
        html,
        has_footnote_block,
        has_toc_block,
        internal_links,
    } = parse_internal(page_info, page_callbacks, settings, tokenization);

    // For producing table of contents indexes
    let mut incrementer = Incrementer(0);

    info!("Finished paragraph gathering, matching on consumption");
    match result {
        Ok(ParseSuccess {
            item: mut elements,
            exceptions,
            ..
        }) => {
            let warnings = extract_exceptions(exceptions);

            info!(
                "Finished parsing, producing final syntax tree ({} warnings)",
                warnings.len(),
            );

            // process_depths() wants a "list type", so we map in a () for each.
            let table_of_contents_depths = table_of_contents_depths
                .into_iter()
                .map(|(depth, contents)| (depth, (), contents));

            // Convert TOC depth lists
            let table_of_contents = process_depths((), table_of_contents_depths)
                .into_iter()
                .map(|(_, items)| build_toc_list_element(&mut incrementer, items))
                .collect::<Vec<_>>();

            // Add a footnote block at the end,
            // if the user doesn't have one already
            if !has_footnote_block {
                info!("No footnote block in elements, appending one");

                elements.push(Element::FootnoteBlock {
                    title: None,
                    hide: false,
                });
            }

            SyntaxTree::from_element_result(
                elements,
                warnings,
                table_of_contents,
                has_toc_block,
                footnotes,
                code,
                html,
                internal_links,
            )
        }
        Err(warning) => {
            // This path is only reachable if a very bad error occurs.
            //
            // If this happens, then just return the input source as the output
            // and the warning.

            error!("Fatal error occurred at highest-level parsing: {warning:#?}");
            let wikitext = tokenization.full_text().inner();
            let elements = vec![text!(wikitext)];
            let warnings = vec![warning];
            let table_of_contents = vec![];
            let footnotes = vec![];
            let internal_links = vec![];

            SyntaxTree::from_element_result(
                elements,
                warnings,
                table_of_contents,
                has_toc_block,
                footnotes,
                code,
                html,
                internal_links,
            )
        }
    }
}

/// Runs the parser, but returns the raw internal results prior to conversion.
pub fn parse_internal<'r, 't>(
    page_info: &'r PageInfo<'t>,
    page_callbacks: Rc<dyn PageCallbacks>,
    settings: &'r WikitextSettings,
    tokenization: &'r Tokenization<'t>,
) -> UnstructuredParseResult<'r, 't>
where
    'r: 't,
{
    let mut parser = Parser::new(tokenization, page_info, page_callbacks, settings);

    // At the top level, we gather elements into paragraphs
    info!("Running parser on tokens");
    let result = gather_paragraphs(&mut parser, RULE_PAGE, NO_CLOSE_CONDITION);

    // Build and return
    let table_of_contents_depths = parser.remove_table_of_contents();
    let footnotes = parser.remove_footnotes();
    let code = parser.remove_code();
    let html = parser.remove_html();
    let internal_links = parser.remove_internal_links();
    let has_footnote_block = parser.has_footnote_block();
    let has_toc_block = parser.has_toc_block();

    UnstructuredParseResult {
        result,
        table_of_contents_depths,
        footnotes,
        code,
        html,
        has_footnote_block,
        has_toc_block,
        internal_links,
    }
}

// Helper functions

fn extract_exceptions(
    exceptions: Vec<ParseException>,
) -> Vec<ParseWarning> {
    let mut warnings = Vec::new();

    for exception in exceptions {
        match exception {
            ParseException::Warning(warning) => {
                if warning.kind() != ParseWarningKind::ManualBreak {
                    warnings.push(warning)
                }
            }
        }
    }

    warnings
}

fn unwrap_toc_list(depth: usize, incr: &mut Incrementer, list: DepthList<(), String>) -> Vec<Element<'static>> {
    let build_item = |item| match item {
        DepthItem::List(_, list) => unwrap_toc_list(depth+1, incr, list),
        DepthItem::Item(name) => {
            let anchor = format!("#toc{}", incr.next());
            let link = Element::Link {
                ltype: LinkType::TableOfContents,
                link: LinkLocation::Url(Cow::Owned(anchor)),
                label: LinkLabel::Text(Cow::Owned(name)),
                target: None,
            };

            let mut attrs = AttributeMap::new();
            attrs.insert("style", Cow::from(format!("margin-left: {}em", depth*2)));

            vec![Element::Container(Container::new(ContainerType::Div, vec![link], attrs))]
        }
    };

    list.into_iter().flat_map(build_item).collect()
}

fn build_toc_list_element(
    incr: &mut Incrementer,
    list: DepthList<(), String>,
) -> Element<'static> {
    let items = unwrap_toc_list(0, incr, list);
    Element::Fragment(items)
}

// Incrementer for TOC

#[derive(Debug)]
struct Incrementer(usize);

impl NextIndex<TableOfContentsIndex> for Incrementer {
    fn next(&mut self) -> usize {
        let index = self.0;
        self.0 += 1;
        index
    }
}

// Parse internal result

#[derive(Serialize, Deserialize, Debug, Clone)]
/// The returned result from parsing.
pub struct UnstructuredParseResult<'r, 't> {
    pub result: ParseResult<'r, 't, Vec<Element<'t>>>,

    /// The "depths" list for table of content entries.
    ///
    /// Each value is a zero-indexed depth of how
    pub table_of_contents_depths: Vec<(usize, String)>,

    /// The list of footnotes.
    ///
    /// Each entry is a series of elements, in combination
    /// they make the contents of one footnote.
    pub footnotes: Vec<Vec<Element<'t>>>,

    // The list of [[code]] elements.
    pub code: Vec<(String, String)>,

    // The list of [[html]] elements.
    pub html: Vec<String>,

    /// Whether a footnote block was placed during parsing.
    pub has_footnote_block: bool,

    /// Whether a TOC block was placed during parsing.
    pub has_toc_block: bool,

    // The list of internal links.
    pub internal_links: Vec<PageRef<'t>>,
}
