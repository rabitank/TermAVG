use crate::pages::pipeline::visual_element::VisualElement;

pub fn clear_animations_by_prefix(elements: &mut [VisualElement], prefix: &str) {
    for ve in elements.iter_mut().filter(|x| x.name.starts_with(prefix)) {
        ve.clear_animation_runtime();
    }
}

pub fn clear_animations_by_name(elements: &mut [VisualElement], name: &str) {
    for ve in elements.iter_mut().filter(|x| x.name == name) {
        ve.clear_animation_runtime();
    }
}
