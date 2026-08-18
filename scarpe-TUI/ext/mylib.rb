require "ffi"
require "rbconfig"

module ScarpeTuiBackend
  extend FFI::Library

  build = ENV.fetch("SCARPE_TUI_BUILD", "debug")
  unless %w[debug release].include?(build)
    raise ArgumentError, "SCARPE_TUI_BUILD must be 'debug' or 'release'"
  end

  library_name =
    case RbConfig::CONFIG["host_os"]
    when /darwin/
      "librust_core.dylib"
    when /linux/
      "librust_core.so"
    when /mswin|mingw|cygwin/
      "rust_core.dll"
    else
      "librust_core"
    end

  library_path = File.expand_path(
    "../rust_core/target/#{build}/#{library_name}",
    __dir__
  )

  ffi_lib library_path

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
  attach_function :scarpe_tui_set_style, [:pointer, :int, :int, :int, :int], :int
  attach_function :scarpe_tui_update_text, [:pointer, :int, :string], :int
end