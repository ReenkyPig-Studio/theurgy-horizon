fn hook(types: &mut Vec<String>) {
 crate::game::theurgy_registry::component_types(types);
 // CodeDefinedGUI 1.14.0 is embedded by the Java mod, not available as a
 // separate Horizon mod. Its persistent filter components must share the
 // stack codec so filters survive insertion, removal and world reloads.
 for id in ["codedefinedgui:list_filter_contents", "codedefinedgui:list_filter_config",
     "codedefinedgui:attribute_filter_reference", "codedefinedgui:attribute_filter_config"] {
  types.push(id.to_owned());
 }
}
