use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::types::IndexedElement;

static IN_MEMORY_CACHE: Mutex<Option<HashMap<usize, Vec<IndexedElement>>>> = Mutex::new(None);

fn cache_dir() -> PathBuf {
    std::env::temp_dir().join("computer-use-cache")
}

fn cache_file(hwnd_id: usize) -> PathBuf {
    cache_dir().join(format!("{}.json", hwnd_id))
}

/// Save indexed elements for a window.
pub fn save_elements(hwnd_id: usize, elements: Vec<IndexedElement>) {
    // Save in memory
    let mut guard = IN_MEMORY_CACHE.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    map.insert(hwnd_id, elements.clone());

    // Also persist to disk for separate process invocations
    let dir = cache_dir();
    let _ = fs::create_dir_all(&dir);
    let path = cache_file(hwnd_id);
    if let Ok(json) = serde_json::to_string(&elements) {
        let _ = fs::write(path, json);
    }
}

/// Look up a specific indexed element for a window.
pub fn get_element(hwnd_id: usize, index: usize) -> Option<IndexedElement> {
    // Try memory
    {
        let guard = IN_MEMORY_CACHE.lock().unwrap();
        if let Some(map) = guard.as_ref() {
            if let Some(elements) = map.get(&hwnd_id) {
                if let Some(elem) = elements.iter().find(|e| e.index == index) {
                    return Some(elem.clone());
                }
            }
        }
    }

    // Try disk cache
    let path = cache_file(hwnd_id);
    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(elements) = serde_json::from_str::<Vec<IndexedElement>>(&content) {
            return elements.into_iter().find(|e| e.index == index);
        }
    }

    None
}
