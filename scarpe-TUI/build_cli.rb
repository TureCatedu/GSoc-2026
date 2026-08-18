require "base64"
require "digest"
require "fileutils"
require "rbconfig"

ROOT = File.expand_path(__dir__)

library_filename =
  case RbConfig::CONFIG.fetch("host_os", "")
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
  warn "  cargo build --release --manifest-path rust_core/Cargo.toml"
  exit 1
end

puts "Bundling Scarpe AI CLI..."

library_bytes = File.binread(release_library)
library_b64 = Base64.strict_encode64(library_bytes)
library_digest = Digest::SHA256.hexdigest(library_bytes)

mylib_src = File.readlines(mylib_path).reject do |line|
  line.match?(/\Arequire ["'](?:ffi|rbconfig)["']\s*\z/)
end.join

scarpe_tui_src = File.readlines(scarpe_tui_path).reject do |line|
  line.match?(/\Arequire_relative .*?\n\z/)
end.join

app_src = File.readlines(app_path).reject do |line|
  line.match?(/\Arequire_relative .*?\n\z/) ||
    line.match?(/\Arequire ["'](?:net\/http|json|fileutils|open3)["']\s*\z/)
end.join

template = <<~RUBY
  #!/usr/bin/env ruby

  begin
    require "ffi"
    require "net/http"
    require "json"
    require "fileutils"
    require "open3"
    require "base64"
    require "digest"
    require "rbconfig"

    CLI_DIR = File.expand_path("~/.scarpe_ai")
    LIBRARY_NAME = #{library_filename.dump}
    LIBRARY_PATH = File.join(CLI_DIR, LIBRARY_NAME)
    FileUtils.mkdir_p(CLI_DIR)

    payload = DATA.read
    expected_digest = #{library_digest.dump}
    unless File.file?(LIBRARY_PATH) &&
           Digest::SHA256.file(LIBRARY_PATH).hexdigest == expected_digest
      File.binwrite(LIBRARY_PATH, Base64.decode64(payload))
      if RUBY_PLATFORM =~ /darwin/
        system("codesign", "-s", "-", "-f", LIBRARY_PATH,
               out: File::NULL, err: File::NULL)
      end
    end

    ENV["SCARPE_TUI_LIB"] = LIBRARY_PATH

  #{mylib_src}
  #{scarpe_tui_src}
  #{app_src}

  rescue StandardError => e
    log_file = File.expand_path("~/.scarpe_ai/error.log")
    FileUtils.mkdir_p(File.dirname(log_file))
    File.write(
      log_file,
      "\#{e.class}: \#{e.message}\\n\#{Array(e.backtrace).join("\\n")}"
    )
    puts "Scarpe AI encountered a fatal error."
    puts "Check the log file at: \#{log_file}"
  end

  __END__
  #{library_b64}
RUBY

output_path = File.join(ROOT, "scarpe")
File.write(output_path, template)
FileUtils.chmod(0o700, output_path)

puts "Executable 'scarpe' generated successfully!"