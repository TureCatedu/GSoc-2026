require "ffi"
require "rbconfig"

module ScarpeTuiBackend
  extend FFI::Library

  def self.platform_library_name
    case RbConfig::CONFIG.fetch("host_os", "")
    when /darwin/
      "librust_core.dylib"
    when /linux/
      "librust_core.so"
    when /mswin|mingw|cygwin/
      "rust_core.dll"
    else
      "librust_core"
    end
  end

  def self.library_candidates
    override = ENV["SCARPE_TUI_LIB"]
    return [File.expand_path(override)] if override && !override.empty?

    name = platform_library_name
    requested_build = ENV["SCARPE_TUI_BUILD"]
    builds =
      if requested_build
        unless %w[release debug].include?(requested_build)
          raise ArgumentError, "SCARPE_TUI_BUILD must be 'release' or 'debug'"
        end
        [requested_build]
      else
        %w[release debug]
      end

    builds.map do |build|
      File.expand_path("../rust_core/target/#{build}/#{name}", __dir__)
    end
  end

  candidates = library_candidates
  library_path = candidates.find { |path| File.file?(path) }

  unless library_path
    raise LoadError, <<~MESSAGE
      Could not find the Scarpe Rust library.
      Attempted:
      #{candidates.map { |path| "  - #{path}" }.join("\n")}
      Build it with:
        cargo build --release --manifest-path rust_core/Cargo.toml
      Or set SCARPE_TUI_LIB to an explicit native library path.
    MESSAGE
  end

  ffi_lib library_path

  attach_function :scarpe_tui_init, [:bool], :pointer
  attach_function :scarpe_tui_free_context, [:pointer], :void
  attach_function :scarpe_tui_render, [:pointer], :int
  attach_function :scarpe_tui_create_node, [:pointer, :int, :string], :int
  attach_function :scarpe_tui_append_child, [:pointer, :int, :int], :int
  attach_function :scarpe_tui_poll_events, [:pointer], :int
  attach_function :scarpe_tui_get_text, [:pointer, :int], :pointer
  attach_function :scarpe_tui_free_string, [:pointer], :void
  attach_function :scarpe_tui_get_clicked_button, [:pointer], :int
  attach_function :scarpe_tui_get_checkbox_state, [:pointer, :int], :int
  attach_function :scarpe_tui_set_style, [:pointer, :int, :int, :int, :int], :int
  attach_function :scarpe_tui_update_text, [:pointer, :int, :string], :int
end