require 'ffi'

module ScarpeTuiBackend
  extend FFI::Library
    
  lib_path = File.expand_path("../rust_core/target/debug/librust_core.dylib", __dir__)
  ffi_lib lib_path

  attach_function :scarpe_tui_init, [:bool], :pointer
  attach_function :scarpe_tui_free_context, [:pointer], :void
  attach_function :scarpe_tui_render, [:pointer], :int
  attach_function :scarpe_tui_create_node, [:pointer, :int, :string], :int
  attach_function :scarpe_tui_append_child, [:pointer, :int, :int], :int
  attach_function :scarpe_tui_poll_events, [:pointer], :int
  attach_function :scarpe_tui_get_text, [:pointer, :int], :pointer
  attach_function :scarpe_tui_free_string, [:uint64], :void
  attach_function :scarpe_tui_get_clicked_button, [:pointer], :int
  attach_function :scarpe_tui_get_checkbox_state, [:pointer, :int], :int
  attach_function :scarpe_tui_get_checkbox_state, [:pointer, :int], :int
  attach_function :scarpe_tui_set_style, [:pointer, :int, :int, :int, :int], :int
end