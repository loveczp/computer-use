use uiautomation::patterns::UIValuePattern;
use uiautomation::types::Handle;
use uiautomation::UIAutomation;
use windows::Win32::Foundation::HWND;

use super::desktop::ensure_user_desktop;
use crate::types::{AccessibilityState, IndexedElement, WindowRect};

/// Traverses the UI Automation accessibility tree for a given window.
pub fn inspect_accessibility(
    hwnd: HWND,
) -> Result<(AccessibilityState, Vec<IndexedElement>), Box<dyn std::error::Error + Send + Sync>> {
    ensure_user_desktop()?;

    let uia = UIAutomation::new()?;
    let root = uia.element_from_handle(Handle::from(hwnd.0 as isize))?;

    let mut elements: Vec<IndexedElement> = Vec::new();
    let mut tree_lines: Vec<String> = Vec::new();
    let mut focused_line: Option<String> = None;
    let mut document_text: Option<String> = None;

    let walker = uia.get_control_view_walker()?;

    // Recursive helper function
    fn walk_element(
        walker: &uiautomation::UITreeWalker,
        element: &uiautomation::UIElement,
        depth: usize,
        index_counter: &mut usize,
        elements: &mut Vec<IndexedElement>,
        tree_lines: &mut Vec<String>,
        focused_line: &mut Option<String>,
        document_text: &mut Option<String>,
    ) {
        let index = *index_counter;
        *index_counter += 1;

        let name = element.get_name().unwrap_or_default();
        let class_name = element.get_classname().unwrap_or_default();
        let control_type = element
            .get_control_type()
            .map(|t| format!("{:?}", t))
            .unwrap_or_else(|_| "Unknown".to_string());

        let rect_raw = element.get_bounding_rectangle().unwrap_or_default();
        let rect = WindowRect {
            x: rect_raw.get_left(),
            y: rect_raw.get_top(),
            width: rect_raw.get_width(),
            height: rect_raw.get_height(),
        };

        let is_focused = element.has_keyboard_focus().unwrap_or(false);
        let is_enabled = element.is_enabled().unwrap_or(true);

        let value = if let Ok(val_pattern) = element.get_pattern::<UIValuePattern>() {
            val_pattern.get_value().ok().filter(|v| !v.is_empty())
        } else {
            None
        };

        let mut desc = format!(
            "[{}] {}: \"{}\"",
            index,
            control_type,
            if name.is_empty() { &class_name } else { &name }
        );

        if let Some(ref v) = value {
            let preview = if v.len() > 60 {
                format!("{}...", &v[..60])
            } else {
                v.clone()
            };
            desc.push_str(&format!(" (value: \"{}\")", preview.replace('\n', " ")));
            if document_text.is_none() && (control_type == "Edit" || control_type == "Document") {
                *document_text = Some(v.clone());
            }
        }

        if is_focused {
            desc.push_str(" [focused]");
            if focused_line.is_none() {
                *focused_line = Some(desc.clone());
            }
        }

        if !is_enabled {
            desc.push_str(" [disabled]");
        }

        let indent = "  ".repeat(depth);
        tree_lines.push(format!("{}{}", indent, desc));

        elements.push(IndexedElement {
            index,
            name,
            control_type,
            class_name,
            rect,
            is_focused,
            is_enabled,
            value,
        });

        // Max elements limit to avoid infinite trees
        if elements.len() >= 500 {
            return;
        }

        // Walk children
        if let Ok(first_child) = walker.get_first_child(element) {
            let mut current = Ok(first_child);
            while let Ok(ref child) = current {
                walk_element(
                    walker,
                    child,
                    depth + 1,
                    index_counter,
                    elements,
                    tree_lines,
                    focused_line,
                    document_text,
                );
                if elements.len() >= 500 {
                    break;
                }
                current = walker.get_next_sibling(child);
            }
        }
    }

    let mut counter = 1;
    walk_element(
        &walker,
        &root,
        0,
        &mut counter,
        &mut elements,
        &mut tree_lines,
        &mut focused_line,
        &mut document_text,
    );

    let state = AccessibilityState {
        tree: tree_lines.join("\n"),
        focused_element: focused_line,
        selected_text: None,
        selected_elements: None,
        document_text,
    };

    Ok((state, elements))
}
