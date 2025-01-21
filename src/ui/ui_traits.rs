pub(crate) trait ExpirableElement {
    fn is_alive(&self) -> bool;
}