require "base64"
require "digest"
require "fileutils"
require "rbconfig"

ROOT = File.expand_path(__dir__)

library_filename =
  case RbConfig::CONFIG["host_os"]
  when /darwin/
    "librust_core.dylib"
  when /linux/
    "librust_core.so"
  when /mswin|mingw|cygwin/
    "rust_core.dll"
  else
    abort "Unsupported platform: #{RbConfig::CONFIG['host_os']}"
  end

release_library = File.join(ROOT, "rust_core", "target", "release", library_filename)
mylib_path = File.join(ROOT, "ext", "mylib.rb")
scarpe_tui_path = File.join(ROOT, "lib", "scarpe_tui.rb")
app_path = File.join(ROOT, "examples", "app.rb")

unless File.file?(release_library)
  warn "Error: Compile Rust project first with:"
  warn "  cd rust_core && cargo build --release"
  exit 1
end

puts "Bundling Scarpe AI CLI..."

library_bytes = File.binread(release_library)
library_b64 = Base64.strict_encode64(library_bytes)
library_size = library_bytes.bytesize
library_digest = Digest::SHA256.hexdigest(library_bytes)

mylib_src = File.read(mylib_path)
  .gsub(/require ["']ffi["']\s*/, "")
  .gsub(/require ["']rbconfig["']\s*/, "")
  .gsub(/module ScarpeTuiBackend.*?^end/m, <<~RUBY.chomp)
    module ScarpeTuiBackend
      extend FFI::Library
      ffi_lib LIBRARY_PATH

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
  RUBY

scarpe_tui_src = File.read(scarpe_tui_path)
  .gsub(/require_relative .*?\n/, "")

app_src = File.read(app_path)
  .gsub(/require_relative .*?\n/, "")
  .gsub(/require ["']net\/http["']\n/, "")
  .gsub(/require ["']json["']\n/, "")
  .gsub(/require ["']fileutils["']\n/, "")
  .gsub(/exec\("ruby \#\{__FILE__\}", "--setup"\)/, 'exec(RbConfig.ruby, __FILE__, "--setup")')
  .gsub(/exec\("ruby \#\{__FILE__\}"\)/, "exec(RbConfig.ruby, __FILE__)")

template = <<~RUBY
  #!/usr/bin/env ruby

  begin
    require "ffi"
    require "net/http"
    require "json"
    require "fileutils"
    require "base64"
    require "digest"
    require "rbconfig"

    CLI_DIR = File.expand_path("~/.scarpe_ai")
    LIBRARY_NAME = #{library_filename.dump}
    LIBRARY_PATH = File.join(CLI_DIR, LIBRARY_NAME)
    FileUtils.mkdir_p(CLI_DIR)

    payload = DATA.read
    if !File.exist?(LIBRARY_PATH) ||
        File.size(LIBRARY_PATH) != #{library_size} ||
        Digest::SHA256.file(LIBRARY_PATH).hexdigest != #{library_digest.dump}
      File.binwrite(LIBRARY_PATH, Base64.decode64(payload))
      if RUBY_PLATFORM =~ /darwin/
        system("codesign", "-s", "-", "-f", LIBRARY_PATH,
               out: File::NULL, err: File::NULL)
      end
    end

  #{mylib_src}
  #{scarpe_tui_src}
  #{app_src}

  rescue Exception => e
    log_file = File.expand_path("~/.scarpe_ai/error.log")
    FileUtils.mkdir_p(File.dirname(log_file))
    File.write(log_file, "\#{e.class}: \#{e.message}\\n\#{e.backtrace.join("\\n")}")
    puts "Scarpe AI encountered a fatal error."
    puts "Check the log file at: \#{log_file}"
  end

  __END__
  #{library_b64}
RUBY

output_path = File.join(ROOT, "scarpe")
File.write(output_path, template)
FileUtils.chmod("+x", output_path)

puts "Executable 'scarpe' generated successfully!"