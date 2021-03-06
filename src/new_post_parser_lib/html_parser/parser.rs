use crate::html_parser::node::Node;
use std::str;
use std::collections::{HashSet};
use linked_hash_map::LinkedHashMap;
use crate::{Element, HtmlParser};

lazy_static! {
  static ref VOID_ELEMENTS: HashSet<&'static str> = {
    let mut set = HashSet::new();

    set.insert("area");
    set.insert("base");
    set.insert("br");
    set.insert("wbr");
    set.insert("col");
    set.insert("hr");
    set.insert("img");
    set.insert("input");
    set.insert("link");
    set.insert("meta");
    set.insert("param");

    return set;
  };
}

impl HtmlParser {
  pub fn new() -> HtmlParser {
    return HtmlParser {};
  }

  pub fn parse(&self, html: &str) -> Result<Vec<Node>, &str> {
    let (result_nodes, _) = self.parse_internal(
      &html.encode_utf16().collect::<Vec<u16>>(),
      0,
    );

    return Result::Ok(result_nodes);
  }

  //noinspection DuplicatedCode
  fn parse_internal(&self, html: &Vec<u16>, start: usize) -> (Vec<Node>, usize) {
    let mut local_offset = start;
    let mut out_nodes: Vec<Node> = Vec::with_capacity(16);
    let mut current_buffer: Vec<u16> = Vec::with_capacity(16);

    while local_offset < html.len() {
      let curr_char = html[local_offset as usize] as u16;

      if curr_char == '<' as u16 {
        if current_buffer.len() > 0 {
          let u16_string = String::from_utf16_lossy(&current_buffer.as_slice());

          out_nodes.push(Node::Text(u16_string));
          current_buffer.clear();
        }

        local_offset += 1;

        let next_char = html[local_offset as usize] as u16;
        if next_char == '/' as u16 {
          let offset = self.skip_tag_end(html, local_offset);
          local_offset = offset;

          return (out_nodes, local_offset);
        }

        let (element, offset) = self.parse_tag(html, local_offset);
        out_nodes.push(Node::Element(element));
        local_offset = offset;

        continue;
      }

      current_buffer.push(curr_char);
      local_offset += 1;
    }

    if current_buffer.len() > 0 {
      let u16_string = String::from_utf16_lossy(&current_buffer.as_slice());

      out_nodes.push(Node::Text(u16_string));
      current_buffer.clear();
    }

    return (out_nodes, local_offset);
  }

  fn parse_tag(&self, html: &Vec<u16>, start: usize) -> (Element, usize) {
    let mut local_offset = start;
    let mut tag_raw: Vec<u16> = Vec::with_capacity(32);

    while local_offset < html.len() {
      let ch = html[local_offset as usize] as u16;
      if ch == '>' as u16 {
        break;
      }

      tag_raw.push(ch);
      local_offset += 1;
    }

    // Skip the ">"
    local_offset += 1;

    let element = self.create_tag(&String::from_utf16_lossy(tag_raw.as_slice()));
    if element.is_void_element {
      return (element, local_offset);
    }

    let (child_nodes, new_offset) = self.parse_internal(
      html,
      local_offset,
    );

    let updated_element = Element {
      tag_name: element.tag_name,
      attributes: element.attributes,
      children: child_nodes,
      is_void_element: false,
    };

    return (updated_element, new_offset);
  }

  fn skip_tag_end(&self, html: &Vec<u16>, start: usize) -> usize {
    let mut local_offset = start;

    while local_offset < html.len() {
      let ch = html[local_offset as usize] as u16;
      if ch == '>' as u16 {
        return local_offset + 1;
      }

      local_offset += 1;
    }

    panic!("Failed to find tag end");
  }

  fn create_tag(&self, tag_raw: &String) -> Element {
    let tag_parts = self.split_into_parts_by_separator(&tag_raw, ' ' as u16);
    if tag_parts.is_empty() {
      panic!("tag_parts is empty! tag_raw={}", tag_raw);
    }

    let mut tag_name_maybe: Option<String> = Option::None;
    let mut attributes: LinkedHashMap<String, String> = LinkedHashMap::new();

    for tag_part in tag_parts {
      if !tag_part.contains("=") {
        tag_name_maybe = Option::Some(String::from(tag_part));
        continue;
      }

      let attribute_split_vec = self.split_into_parts_by_separator(&tag_part, '=' as u16);
      let attr_name = attribute_split_vec[0].as_str();
      let mut attr_value = attribute_split_vec[1].as_str();

      if attr_value.starts_with('\"') {
        attr_value = &attr_value[1..]
      }

      if attr_value.ends_with("\"") {
        attr_value = &attr_value[..(attr_value.len() - 1)]
      }

      if attr_name.is_empty() || attr_value.is_empty() {
        continue;
      }

      attributes.insert(String::from(attr_name), String::from(attr_value));
    }

    if tag_name_maybe.is_none() {
      panic!("Tag has no name!")
    }

    let tag_name = tag_name_maybe.unwrap();
    let is_void_element = VOID_ELEMENTS.contains(&tag_name.as_str());

    return Element {
      tag_name: tag_name,
      attributes,
      children: Vec::with_capacity(4),
      is_void_element
    };
  }

  fn split_into_parts_by_separator(&self, tag_raw: &String, separator: u16) -> Vec<String> {
    let mut is_inside_string = false;
    let mut offset: usize = 0;
    let mut tag_parts: Vec<String> = Vec::with_capacity(4);
    let mut current_tag_part: Vec<u16> = Vec::with_capacity(16);
    let tag_bytes = tag_raw.encode_utf16().collect::<Vec<u16>>();

    while offset < tag_bytes.len() {
      let ch = tag_bytes[offset as usize] as u16;

      if ch == '\"' as u16 {
        is_inside_string = !is_inside_string;
      }

      if ch == separator && !is_inside_string {
        let u16_string = String::from_utf16_lossy(&current_tag_part.as_slice());
        tag_parts.push(u16_string.clone());
        current_tag_part.clear();

        offset += 1;
        continue;
      }

      current_tag_part.push(ch);
      offset += 1;
    }

    if current_tag_part.len() > 0 {
      let u16_string = String::from_utf16_lossy(&current_tag_part.as_slice());
      tag_parts.push(u16_string.clone());
      current_tag_part.clear();
    }

    return tag_parts;
  }

  // Debug stuff

  #[allow(dead_code)]
  pub fn debug_print_nodes(&self, nodes: &Vec<Node>) {
    self.debug_print_nodes_internal(
      nodes,
      &mut |node_string: String| { println!("{}", node_string) }
    );
  }

  #[allow(dead_code)]
  fn debug_print_nodes_internal(&self, nodes: &Vec<Node>, iterator: &mut dyn FnMut(String)) {
    for node in nodes {
      match node {
        Node::Text(text) => {
          iterator(format!("{}", text));
        }
        Node::Element(element) => {
          iterator(format!("<{}{}>", &element.tag_name, self.debug_format_attributes(&element.attributes)));
          self.debug_print_nodes_internal(&element.children, iterator);
        }
      }
    }
  }

  #[allow(dead_code)]
  pub fn debug_concat_into_string(&self, nodes: &Vec<Node>) -> String {
    let mut result_string = String::new();

    self.debug_concat_into_string_internal(
      nodes,
      &mut |node_string: String| {
        result_string.push_str(format!("{}\n", node_string.as_str()).as_str())
      }
    );

    return result_string;
  }

  #[allow(dead_code)]
  pub fn debug_concat_into_string_internal(&self, nodes: &Vec<Node>, iterator: &mut dyn FnMut(String)) {
    for node in nodes {
      match node {
        Node::Text(text) => {
          iterator(format!("{}", text));
        }
        Node::Element(element) => {
          iterator(format!("<{}{}>", &element.tag_name, self.debug_format_attributes(&element.attributes)));
          self.debug_concat_into_string_internal(&element.children, iterator);
        }
      }
    }
  }

  #[allow(dead_code)]
  fn debug_format_attributes(&self, attributes: &LinkedHashMap<String, String>) -> String {
    let mut result_string = String::new();

    if attributes.is_empty() {
      return result_string;
    }

    for (attr_key, attr_value) in attributes {
      result_string.push_str(format!(", {}={}", attr_key, attr_value).as_str());
    }

    return result_string;
  }
}